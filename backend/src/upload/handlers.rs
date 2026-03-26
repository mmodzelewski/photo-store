use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use std::collections::HashSet;
use time::OffsetDateTime;
use tracing::{debug, error, info, warn};

use crate::AppState;
use crate::error::{Error, Result};
use crate::file::repository::{DbFileRepository, FileRepository};
use crate::file::{File, FileState};
use crate::session::Session;
use crate::ulid::Id;

use super::repository::{DbUploadRepository, UploadRepository};
use super::{
    ChunkUrl, CompleteUploadRequest, CompleteUploadResponse, InitUploadRequest, InitUploadResponse,
    MAX_CHUNK_SIZE, MAX_PARTS, MIN_CHUNK_SIZE, REQUIRED_THUMBNAILS, ThumbnailUrl, UploadSession,
    UploadStatusResponse,
};

pub(super) async fn init_upload(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
    Json(request): Json<InitUploadRequest>,
) -> Result<(StatusCode, Json<InitUploadResponse>)> {
    debug!(%file_id, "Initializing upload session");

    let file_repo = DbFileRepository {
        db: state.db.clone(),
    };
    let file = load_and_authorize(&file_repo, &file_id, session.user_id()).await?;
    let mut upload_repo = DbUploadRepository {
        db: state.db.clone(),
    };

    let existing_session = upload_repo.find_session(&file_id).await?;
    let s3_assembled =
        if matches!(file.state, FileState::SyncInProgress) && existing_session.is_none() {
            check_upload_complete(&state, &file).await?
        } else {
            false
        };

    match decide_init_action(
        &file.state,
        existing_session.as_ref(),
        OffsetDateTime::now_utc(),
        s3_assembled,
    ) {
        InitAction::RejectAlreadySynced => {
            error!(%file_id, "File is already synced");
            return Err(Error::UploadConflict);
        }
        InitAction::ReturnExistingSession => {
            let session = existing_session.unwrap();
            debug!(%file_id, "Returning existing upload session");
            let response = build_init_response(&state, &file, &session).await?;
            return Ok((StatusCode::OK, Json(response)));
        }
        InitAction::CleanupExpiredAndProceed => {
            let session = existing_session.unwrap();
            warn!(%file_id, "Cleaning up expired session before re-init");
            cleanup_s3_upload(&state, &file, &session).await;
            upload_repo.delete_session(&file_id).await?;
        }
        InitAction::CrashRecoveryTransitionToSynced => {
            info!(%file_id, "S3 object already assembled, transitioning to Synced");
            file_repo.update_state(&file_id, FileState::Synced).await?;
            return Err(Error::UploadConflict);
        }
        InitAction::ProceedWithNewUpload => {}
    }

    validate_init_request(&request, state.config.upload.max_file_size)?;

    let chunk_size = request.chunk_size as i64;
    let total_chunks = ((request.total_size + chunk_size - 1) / chunk_size) as i32;

    let session_count = upload_repo.count_user_sessions(&session.user_id()).await?;
    if session_count >= state.config.upload.max_concurrent_sessions {
        error!(
            %file_id,
            count = session_count,
            "Too many concurrent upload sessions"
        );
        return Err(Error::TooManyRequests);
    }

    let s3_key = s3_original_key(&file);
    let bucket = &state.config.storage.bucket_name;

    let create_output = state
        .s3_client
        .create_multipart_upload()
        .bucket(bucket)
        .key(&s3_key)
        .content_type("application/octet-stream")
        .send()
        .await
        .map_err(|e| {
            error!(%file_id, error = %e, "Failed to create S3 multipart upload");
            Error::Storage
        })?;

    let upload_id = create_output
        .upload_id()
        .ok_or_else(|| {
            error!(%file_id, "S3 did not return upload_id");
            Error::Storage
        })?
        .to_string();

    let now = OffsetDateTime::now_utc();
    let ttl = time::Duration::hours(state.config.upload.session_ttl_hours);
    let expires_at = now + ttl;

    let session = UploadSession {
        file_id,
        upload_id,
        total_size: request.total_size,
        chunk_size: request.chunk_size,
        total_chunks,
        created_at: now,
        expires_at,
    };

    upload_repo.create_session(&session).await?;
    file_repo
        .update_state(&file_id, FileState::SyncInProgress)
        .await?;

    let response = build_init_response(&state, &file, &session).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub(super) async fn upload_status(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
) -> Result<Json<UploadStatusResponse>> {
    debug!(%file_id, "Querying upload status");

    let file_repo = DbFileRepository {
        db: state.db.clone(),
    };
    let file = load_and_authorize(&file_repo, &file_id, session.user_id()).await?;

    let upload_repo = DbUploadRepository {
        db: state.db.clone(),
    };
    let session = upload_repo.find_session(&file_id).await?.ok_or_else(|| {
        error!(%file_id, "Upload session not found");
        Error::UploadNotFound
    })?;

    let bucket = &state.config.storage.bucket_name;
    let s3_key = s3_original_key(&file);

    let received_parts =
        list_s3_parts(&state.s3_client, bucket, &s3_key, &session.upload_id).await?;
    let received_set: HashSet<i32> = received_parts.iter().copied().collect();

    let presigning = presigning_config(&state)?;
    let mut missing = Vec::new();
    for part_number in 1..=session.total_chunks {
        if !received_set.contains(&part_number) {
            let url = presign_upload_part(
                &state.s3_client,
                bucket,
                &s3_key,
                &session.upload_id,
                part_number,
                &presigning,
            )
            .await?;
            missing.push(ChunkUrl { part_number, url });
        }
    }

    let (thumbnails_received, thumbnails_missing) =
        check_thumbnails(&state, &file, &presigning).await?;

    Ok(Json(UploadStatusResponse {
        total_chunks: session.total_chunks,
        chunk_size: session.chunk_size,
        total_size: session.total_size,
        received: received_parts,
        missing,
        thumbnails_received,
        thumbnails_missing,
        expires_at: session.expires_at,
    }))
}

pub(super) async fn complete_upload(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<Json<CompleteUploadResponse>> {
    debug!(%file_id, "Completing upload");

    let file_repo = DbFileRepository {
        db: state.db.clone(),
    };
    let file = load_and_authorize(&file_repo, &file_id, session.user_id()).await?;

    let upload_repo = DbUploadRepository {
        db: state.db.clone(),
    };
    let session = upload_repo.find_session(&file_id).await?.ok_or_else(|| {
        error!(%file_id, "Upload session not found");
        Error::UploadNotFound
    })?;

    if session.expires_at <= OffsetDateTime::now_utc() {
        error!(%file_id, "Upload session expired");
        return Err(Error::UploadExpired);
    }

    validate_complete_request(&request, &session)?;

    let missing_thumbs = find_missing_thumbnails(&state, &file).await?;
    if !missing_thumbs.is_empty() {
        error!(
            %file_id,
            missing = ?missing_thumbs,
            "Missing required thumbnails"
        );
        return Err(Error::UploadIncomplete);
    }

    let s3_key = s3_original_key(&file);
    let bucket = &state.config.storage.bucket_name;

    let mut sorted_parts = request.parts;
    sorted_parts.sort_by_key(|p| p.part_number);

    let completed_parts: Vec<CompletedPart> = sorted_parts
        .iter()
        .map(|p| {
            CompletedPart::builder()
                .part_number(p.part_number)
                .e_tag(&p.etag)
                .build()
        })
        .collect();

    let multipart_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    state
        .s3_client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(&s3_key)
        .upload_id(&session.upload_id)
        .multipart_upload(multipart_upload)
        .send()
        .await
        .map_err(|e| {
            error!(%file_id, error = %e, "S3 CompleteMultipartUpload failed");
            Error::Storage
        })?;

    file_repo.update_state(&file_id, FileState::Synced).await?;
    upload_repo.delete_session(&file_id).await?;

    info!(%file_id, "Upload completed successfully");
    Ok(Json(CompleteUploadResponse { file_id }))
}

pub(super) async fn abort_upload(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
) -> Result<StatusCode> {
    debug!(%file_id, "Aborting upload");

    let file_repo = DbFileRepository {
        db: state.db.clone(),
    };
    let file = load_and_authorize(&file_repo, &file_id, session.user_id()).await?;

    let upload_repo = DbUploadRepository {
        db: state.db.clone(),
    };
    let session = match upload_repo.find_session(&file_id).await? {
        Some(s) => s,
        None => {
            debug!(%file_id, "No upload session found, returning 204");
            return Ok(StatusCode::NO_CONTENT);
        }
    };

    cleanup_s3_upload(&state, &file, &session).await;
    upload_repo.delete_session(&file_id).await?;
    file_repo.update_state(&file_id, FileState::Failed).await?;

    info!(%file_id, "Upload aborted");
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn cleanup_expired_uploads(state: AppState) {
    let interval_secs = state.config.upload.gc_interval_secs;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        let now = OffsetDateTime::now_utc();
        let repo = DbUploadRepository {
            db: state.db.clone(),
        };

        let expired = match repo.find_expired_sessions(now).await {
            Ok(sessions) => sessions,
            Err(e) => {
                error!(error = %e, "Failed to query expired upload sessions");
                continue;
            }
        };

        if expired.is_empty() {
            continue;
        }

        info!(count = expired.len(), "Cleaning up expired upload sessions");

        for session in &expired {
            let file_repo = DbFileRepository {
                db: state.db.clone(),
            };

            let file = match file_repo.find(&session.file_id).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    warn!(file_id = %session.file_id, "File not found for expired session, deleting session");
                    let _ = repo.delete_session(&session.file_id).await;
                    continue;
                }
                Err(e) => {
                    error!(file_id = %session.file_id, error = %e, "Failed to load file for cleanup");
                    continue;
                }
            };

            cleanup_s3_upload(&state, &file, session).await;

            if let Err(e) = repo.delete_session(&session.file_id).await {
                error!(file_id = %session.file_id, error = %e, "Failed to delete expired session");
                continue;
            }

            if let Err(e) = file_repo
                .update_state(&session.file_id, FileState::Failed)
                .await
            {
                error!(file_id = %session.file_id, error = %e, "Failed to update file state after cleanup");
                continue;
            }

            info!(file_id = %session.file_id, "Cleaned up expired upload session");
        }
    }
}

#[derive(Debug, PartialEq)]
enum InitAction {
    RejectAlreadySynced,
    ReturnExistingSession,
    CleanupExpiredAndProceed,
    CrashRecoveryTransitionToSynced,
    ProceedWithNewUpload,
}

fn decide_init_action(
    file_state: &FileState,
    session: Option<&UploadSession>,
    now: OffsetDateTime,
    s3_object_assembled: bool,
) -> InitAction {
    match file_state {
        FileState::Synced => InitAction::RejectAlreadySynced,
        FileState::SyncInProgress => match session {
            Some(session) if session.expires_at > now => InitAction::ReturnExistingSession,
            Some(_) => InitAction::CleanupExpiredAndProceed,
            None if s3_object_assembled => InitAction::CrashRecoveryTransitionToSynced,
            None => InitAction::ProceedWithNewUpload,
        },
        FileState::New | FileState::Failed => InitAction::ProceedWithNewUpload,
    }
}

async fn load_and_authorize(repo: &impl FileRepository, file_id: &Id, user_id: Id) -> Result<File> {
    let file = repo.find(file_id).await?.ok_or_else(|| {
        error!(%file_id, "File not found");
        Error::FileNotFound
    })?;

    if file.uploader_id != user_id {
        error!(%file_id, "Upload authorization mismatch");
        return Err(Error::Forbidden);
    }

    Ok(file)
}

fn validate_init_request(request: &InitUploadRequest, max_file_size: i64) -> Result<()> {
    if request.total_size <= 0 || request.total_size > max_file_size {
        error!(total_size = request.total_size, "total_size out of range");
        return Err(Error::FileUpload);
    }

    let chunk_size = request.chunk_size as i64;

    if request.total_size < MIN_CHUNK_SIZE {
        if chunk_size != request.total_size {
            error!(
                chunk_size,
                total_size = request.total_size,
                "For small files, chunk_size must equal total_size"
            );
            return Err(Error::FileUpload);
        }
    } else if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) {
        error!(chunk_size, "chunk_size out of range");
        return Err(Error::FileUpload);
    }

    let total_chunks = (request.total_size + chunk_size - 1) / chunk_size;
    if total_chunks > MAX_PARTS {
        error!(total_chunks, "Too many parts");
        return Err(Error::FileUpload);
    }

    Ok(())
}

fn validate_complete_request(
    request: &CompleteUploadRequest,
    session: &UploadSession,
) -> Result<()> {
    if request.parts.len() as i32 != session.total_chunks {
        error!(
            expected = session.total_chunks,
            actual = request.parts.len(),
            "Part count mismatch"
        );
        return Err(Error::UploadIncomplete);
    }

    let mut seen = HashSet::new();
    for part in &request.parts {
        if part.part_number < 1 || part.part_number > session.total_chunks {
            error!(
                part_number = part.part_number,
                total_chunks = session.total_chunks,
                "Part number out of range"
            );
            return Err(Error::FileUpload);
        }
        if !seen.insert(part.part_number) {
            error!(part_number = part.part_number, "Duplicate part number");
            return Err(Error::FileUpload);
        }
        if part.etag.is_empty() {
            error!(part_number = part.part_number, "Empty ETag");
            return Err(Error::FileUpload);
        }
    }

    Ok(())
}

fn s3_original_key(file: &File) -> String {
    format!("files/{}/{}/original", file.owner_id, file.id)
}

fn s3_thumbnail_key(file: &File, variant: &str) -> String {
    format!("files/{}/{}/{}", file.owner_id, file.id, variant)
}

fn presigning_config(state: &AppState) -> Result<PresigningConfig> {
    PresigningConfig::expires_in(std::time::Duration::from_secs(
        state.config.upload.presigned_url_ttl_secs,
    ))
    .map_err(|e| {
        error!(error = %e, "Invalid presigning configuration");
        Error::Storage
    })
}

async fn presign_upload_part(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: i32,
    config: &PresigningConfig,
) -> Result<String> {
    let presigned = client
        .upload_part()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(part_number)
        .presigned(config.clone())
        .await
        .map_err(|e| {
            error!(part_number, error = %e, "Failed to presign UploadPart");
            Error::Storage
        })?;

    Ok(presigned.uri().to_string())
}

async fn presign_put_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    config: &PresigningConfig,
) -> Result<String> {
    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .presigned(config.clone())
        .await
        .map_err(|e| {
            error!(key, error = %e, "Failed to presign PutObject");
            Error::Storage
        })?;

    Ok(presigned.uri().to_string())
}

async fn build_init_response(
    state: &AppState,
    file: &File,
    session: &UploadSession,
) -> Result<InitUploadResponse> {
    let bucket = &state.config.storage.bucket_name;
    let s3_key = s3_original_key(file);
    let presigning = presigning_config(state)?;

    let mut chunk_urls = Vec::with_capacity(session.total_chunks as usize);
    for part_number in 1..=session.total_chunks {
        let url = presign_upload_part(
            &state.s3_client,
            bucket,
            &s3_key,
            &session.upload_id,
            part_number,
            &presigning,
        )
        .await?;
        chunk_urls.push(ChunkUrl { part_number, url });
    }

    let mut thumbnail_urls = Vec::with_capacity(REQUIRED_THUMBNAILS.len());
    for variant in REQUIRED_THUMBNAILS {
        let key = s3_thumbnail_key(file, variant);
        let url = presign_put_object(&state.s3_client, bucket, &key, &presigning).await?;
        thumbnail_urls.push(ThumbnailUrl {
            variant: variant.to_string(),
            url,
        });
    }

    Ok(InitUploadResponse {
        total_chunks: session.total_chunks,
        chunk_size: session.chunk_size,
        expires_at: session.expires_at,
        chunk_urls,
        thumbnail_urls,
    })
}

async fn list_s3_parts(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
) -> Result<Vec<i32>> {
    let mut received = Vec::new();
    let mut marker: Option<String> = None;

    loop {
        let mut req = client
            .list_parts()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id);

        if let Some(m) = &marker {
            req = req.part_number_marker(m);
        }

        let output = req.send().await.map_err(|e| {
            error!(error = %e, "Failed to list S3 parts");
            Error::Storage
        })?;

        for part in output.parts() {
            if let Some(num) = part.part_number() {
                received.push(num);
            }
        }

        if output.is_truncated().unwrap_or(false) {
            marker = output.next_part_number_marker().map(|s| s.to_string());
            if marker.is_none() {
                break;
            }
        } else {
            break;
        }
    }

    received.sort_unstable();
    Ok(received)
}

async fn head_object_exists(client: &aws_sdk_s3::Client, bucket: &str, key: &str) -> Result<bool> {
    match client.head_object().bucket(bucket).key(key).send().await {
        Ok(_) => Ok(true),
        Err(err) => {
            if err.as_service_error().is_some_and(|e| e.is_not_found()) {
                Ok(false)
            } else {
                error!(key, "S3 HeadObject failed");
                Err(Error::Storage)
            }
        }
    }
}

async fn find_missing_thumbnails(state: &AppState, file: &File) -> Result<Vec<String>> {
    let bucket = &state.config.storage.bucket_name;
    let mut missing = Vec::new();

    for variant in REQUIRED_THUMBNAILS {
        let key = s3_thumbnail_key(file, variant);
        if !head_object_exists(&state.s3_client, bucket, &key).await? {
            missing.push(variant.to_string());
        }
    }

    Ok(missing)
}

async fn check_thumbnails(
    state: &AppState,
    file: &File,
    presigning: &PresigningConfig,
) -> Result<(Vec<String>, Vec<ThumbnailUrl>)> {
    let bucket = &state.config.storage.bucket_name;
    let mut received = Vec::new();
    let mut missing = Vec::new();

    for variant in REQUIRED_THUMBNAILS {
        let key = s3_thumbnail_key(file, variant);
        if head_object_exists(&state.s3_client, bucket, &key).await? {
            received.push(variant.to_string());
        } else {
            let url = presign_put_object(&state.s3_client, bucket, &key, presigning).await?;
            missing.push(ThumbnailUrl {
                variant: variant.to_string(),
                url,
            });
        }
    }

    Ok((received, missing))
}

async fn check_upload_complete(state: &AppState, file: &File) -> Result<bool> {
    let bucket = &state.config.storage.bucket_name;
    let s3_key = s3_original_key(file);

    if !head_object_exists(&state.s3_client, bucket, &s3_key).await? {
        return Ok(false);
    }

    let missing = find_missing_thumbnails(state, file).await?;
    Ok(missing.is_empty())
}

async fn cleanup_s3_upload(state: &AppState, file: &File, session: &UploadSession) {
    let bucket = &state.config.storage.bucket_name;
    let s3_key = s3_original_key(file);

    if let Err(e) = state
        .s3_client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(&s3_key)
        .upload_id(&session.upload_id)
        .send()
        .await
    {
        warn!(
            file_id = %file.id,
            error = %e,
            "Failed to abort S3 multipart upload (may already be cleaned up)"
        );
    }

    for variant in REQUIRED_THUMBNAILS {
        let key = s3_thumbnail_key(file, variant);
        if let Err(e) = state
            .s3_client
            .delete_object()
            .bucket(bucket)
            .key(&key)
            .send()
            .await
        {
            warn!(
                file_id = %file.id,
                variant,
                error = %e,
                "Failed to delete thumbnail during cleanup"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::repository::tests::InMemoryFileRepository;
    use crate::upload::{CompletePart, CompleteUploadRequest, InitUploadRequest};
    use time::OffsetDateTime;

    const MIB: i64 = 1024 * 1024;
    const GIB: i64 = 1024 * 1024 * 1024;
    const DEFAULT_MAX_FILE_SIZE: i64 = 10 * GIB;

    fn make_init_request(total_size: i64, chunk_size: i32) -> InitUploadRequest {
        InitUploadRequest {
            total_size,
            chunk_size,
        }
    }

    fn make_file(owner_id: Id, uploader_id: Id, state: FileState) -> File {
        File {
            id: Id::new(),
            path: "/test/photo.jpg".to_string(),
            name: "photo.jpg".to_string(),
            state,
            created_at: OffsetDateTime::now_utc(),
            added_at: OffsetDateTime::now_utc(),
            sha256: "abc123".to_string(),
            owner_id,
            uploader_id,
            enc_key: "key".to_string(),
        }
    }

    fn make_session(file_id: Id, expires_at: OffsetDateTime) -> UploadSession {
        UploadSession {
            file_id,
            upload_id: "test-upload-id".to_string(),
            total_size: 100 * MIB,
            chunk_size: (16 * MIB) as i32,
            total_chunks: 7,
            created_at: OffsetDateTime::now_utc(),
            expires_at,
        }
    }

    fn make_complete_parts(count: i32) -> Vec<CompletePart> {
        (1..=count)
            .map(|i| CompletePart {
                part_number: i,
                etag: format!("\"etag-{}\"", i),
            })
            .collect()
    }

    fn make_session_for_validation(total_chunks: i32) -> UploadSession {
        UploadSession {
            file_id: Id::new(),
            upload_id: "test".to_string(),
            total_size: total_chunks as i64 * 16 * MIB,
            chunk_size: (16 * MIB) as i32,
            total_chunks,
            created_at: OffsetDateTime::now_utc(),
            expires_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn validate_init_rejects_zero_total_size() {
        let req = make_init_request(0, 1);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_rejects_negative_total_size() {
        let req = make_init_request(-1, 1);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_rejects_total_size_exceeding_max() {
        let req = make_init_request(DEFAULT_MAX_FILE_SIZE + 1, (100 * MIB) as i32);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_accepts_total_size_at_max() {
        let req = make_init_request(DEFAULT_MAX_FILE_SIZE, (100 * MIB) as i32);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_init_small_file_rejects_mismatched_chunk() {
        let req = make_init_request(MIB, 512 * 1024);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_small_file_accepts_matching_chunk() {
        let req = make_init_request(MIB, MIB as i32);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_init_rejects_chunk_below_minimum() {
        let req = make_init_request(100 * MIB, (4 * MIB) as i32);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_rejects_chunk_above_maximum() {
        let req = make_init_request(200 * MIB, (101 * MIB) as i32);
        let result = validate_init_request(&req, DEFAULT_MAX_FILE_SIZE);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_accepts_chunk_at_minimum_boundary() {
        let req = make_init_request(100 * MIB, (5 * MIB) as i32);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_init_accepts_chunk_at_maximum_boundary() {
        let req = make_init_request(200 * MIB, (100 * MIB) as i32);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_init_rejects_too_many_parts() {
        // 50 GiB / 5 MiB = 10,240 parts > MAX_PARTS (10,000)
        let large_max = 100 * GIB;
        let req = make_init_request(50 * GIB, (5 * MIB) as i32);
        let result = validate_init_request(&req, large_max);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_init_accepts_exactly_max_parts() {
        // MAX_PARTS * MIN_CHUNK_SIZE = 10,000 * 5 MiB = 50 GiB
        let large_max = 100 * GIB;
        let total_size = MAX_PARTS * MIN_CHUNK_SIZE;
        let req = make_init_request(total_size, MIN_CHUNK_SIZE as i32);
        assert!(validate_init_request(&req, large_max).is_ok());
    }

    #[test]
    fn validate_init_small_file_single_byte() {
        let req = make_init_request(1, 1);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_init_boundary_at_min_chunk_size() {
        // total_size == MIN_CHUNK_SIZE exactly: uses normal chunk validation path
        let req = make_init_request(MIN_CHUNK_SIZE, MIN_CHUNK_SIZE as i32);
        assert!(validate_init_request(&req, DEFAULT_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn validate_complete_rejects_wrong_part_count() {
        let session = make_session_for_validation(5);
        let request = CompleteUploadRequest {
            parts: make_complete_parts(3),
        };
        let result = validate_complete_request(&request, &session);
        assert!(matches!(result, Err(Error::UploadIncomplete)));
    }

    #[test]
    fn validate_complete_rejects_part_number_below_one() {
        let session = make_session_for_validation(2);
        let request = CompleteUploadRequest {
            parts: vec![
                CompletePart {
                    part_number: 0,
                    etag: "\"etag\"".to_string(),
                },
                CompletePart {
                    part_number: 2,
                    etag: "\"etag\"".to_string(),
                },
            ],
        };
        let result = validate_complete_request(&request, &session);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_complete_rejects_part_number_above_total() {
        let session = make_session_for_validation(2);
        let request = CompleteUploadRequest {
            parts: vec![
                CompletePart {
                    part_number: 1,
                    etag: "\"etag\"".to_string(),
                },
                CompletePart {
                    part_number: 3,
                    etag: "\"etag\"".to_string(),
                },
            ],
        };
        let result = validate_complete_request(&request, &session);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_complete_rejects_duplicate_part_number() {
        let session = make_session_for_validation(2);
        let request = CompleteUploadRequest {
            parts: vec![
                CompletePart {
                    part_number: 1,
                    etag: "\"etag-1\"".to_string(),
                },
                CompletePart {
                    part_number: 1,
                    etag: "\"etag-2\"".to_string(),
                },
            ],
        };
        let result = validate_complete_request(&request, &session);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_complete_rejects_empty_etag() {
        let session = make_session_for_validation(1);
        let request = CompleteUploadRequest {
            parts: vec![CompletePart {
                part_number: 1,
                etag: "".to_string(),
            }],
        };
        let result = validate_complete_request(&request, &session);
        assert!(matches!(result, Err(Error::FileUpload)));
    }

    #[test]
    fn validate_complete_accepts_valid_request() {
        let session = make_session_for_validation(3);
        let request = CompleteUploadRequest {
            parts: make_complete_parts(3),
        };
        assert!(validate_complete_request(&request, &session).is_ok());
    }

    #[test]
    fn validate_complete_accepts_single_part() {
        let session = make_session_for_validation(1);
        let request = CompleteUploadRequest {
            parts: make_complete_parts(1),
        };
        assert!(validate_complete_request(&request, &session).is_ok());
    }

    #[test]
    fn validate_complete_accepts_unordered_parts() {
        let session = make_session_for_validation(3);
        let request = CompleteUploadRequest {
            parts: vec![
                CompletePart {
                    part_number: 3,
                    etag: "\"e3\"".to_string(),
                },
                CompletePart {
                    part_number: 1,
                    etag: "\"e1\"".to_string(),
                },
                CompletePart {
                    part_number: 2,
                    etag: "\"e2\"".to_string(),
                },
            ],
        };
        assert!(validate_complete_request(&request, &session).is_ok());
    }

    #[tokio::test]
    async fn authorize_returns_not_found_for_missing_file() {
        let repo = InMemoryFileRepository::new();
        let result = load_and_authorize(&repo, &Id::new(), Id::new()).await;
        assert!(matches!(result, Err(Error::FileNotFound)));
    }

    #[tokio::test]
    async fn authorize_returns_forbidden_for_wrong_user() {
        let user_id = Id::new();
        let other_user_id = Id::new();
        let file = make_file(user_id, user_id, FileState::New);
        let file_id = file.id;
        let repo = InMemoryFileRepository::with_files(vec![file]);

        let result = load_and_authorize(&repo, &file_id, other_user_id).await;
        assert!(matches!(result, Err(Error::Forbidden)));
    }

    #[tokio::test]
    async fn authorize_returns_file_for_correct_user() {
        let user_id = Id::new();
        let file = make_file(user_id, user_id, FileState::New);
        let file_id = file.id;
        let repo = InMemoryFileRepository::with_files(vec![file]);

        let result = load_and_authorize(&repo, &file_id, user_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, file_id);
    }

    #[test]
    fn init_action_rejects_synced_file() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            decide_init_action(&FileState::Synced, None, now, false),
            InitAction::RejectAlreadySynced
        );
    }

    #[test]
    fn init_action_returns_existing_active_session() {
        let now = OffsetDateTime::now_utc();
        let session = make_session(Id::new(), now + time::Duration::hours(1));
        assert_eq!(
            decide_init_action(&FileState::SyncInProgress, Some(&session), now, false),
            InitAction::ReturnExistingSession
        );
    }

    #[test]
    fn init_action_cleans_up_expired_session() {
        let now = OffsetDateTime::now_utc();
        let session = make_session(Id::new(), now - time::Duration::hours(1));
        assert_eq!(
            decide_init_action(&FileState::SyncInProgress, Some(&session), now, false),
            InitAction::CleanupExpiredAndProceed
        );
    }

    #[test]
    fn init_action_crash_recovery_when_s3_complete() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            decide_init_action(&FileState::SyncInProgress, None, now, true),
            InitAction::CrashRecoveryTransitionToSynced
        );
    }

    #[test]
    fn init_action_proceeds_when_no_session_no_s3() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            decide_init_action(&FileState::SyncInProgress, None, now, false),
            InitAction::ProceedWithNewUpload
        );
    }

    #[test]
    fn init_action_proceeds_from_new_state() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            decide_init_action(&FileState::New, None, now, false),
            InitAction::ProceedWithNewUpload
        );
    }

    #[test]
    fn init_action_proceeds_from_failed_state() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            decide_init_action(&FileState::Failed, None, now, false),
            InitAction::ProceedWithNewUpload
        );
    }

    #[test]
    fn s3_original_key_format() {
        let user_id = Id::new();
        let file = make_file(user_id, user_id, FileState::New);
        let key = s3_original_key(&file);
        assert_eq!(key, format!("files/{}/{}/original", file.owner_id, file.id));
    }

    #[test]
    fn s3_thumbnail_key_format() {
        let user_id = Id::new();
        let file = make_file(user_id, user_id, FileState::New);
        let key = s3_thumbnail_key(&file, "512-cover");
        assert_eq!(
            key,
            format!("files/{}/{}/512-cover", file.owner_id, file.id)
        );
    }
}
