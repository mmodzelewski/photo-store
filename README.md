# Photo store

Photo store is a comprehensive media storage application designed to provide
a seamless experience for managing, sharing, and syncing your media files
across multiple platforms.

### Todo

- [x] Cargo workspace for sharing code between apps?
- [ ] End-to-end encryption

#### Backend

- [x] Update AWS dependencies to v1
- [ ] Refactor sending files to R2/S3
- [x] Encrypt files before storing in R2
- [ ] Save thumbnails

#### Desktop

- [ ] Add exif data to images index (original date added)
- [ ] Add option for adding/removing source directories
- [ ] Watch file changes: add, remove, move
- [ ] Generate multiple thumbnails sizes
- [ ] Test performance for storing thumbnails in sqlite
- [ ] Change process for generating thumbnails - maybe on demand?
- [ ] Add custom assets protocol for getting images by id and resolution - 
      when Tauri v2 is available and async protocol implementation is possible
- [ ] Store auth token in keyring      
- [x] Local images indexing
- [x] Thumbnails generation for images
- [x] Pagination for images loading
- [x] Change command for indexing images to async
- [x] Display images in chronological order
- [x] Add prev/next navigation when previwing an image
- [ ] ~~Add [Persisted Scope plugin](https://github.com/tauri-apps/plugins-workspace/tree/v1/plugins/persisted-scope)
      once the [PR](https://github.com/tauri-apps/plugins-workspace/pull/32) is merged~~ -
      no longer needed, it's covered by saving and restoring settings from db

#### Web

- [x] Project set up
- [x] Sign up and login implementation

#### Mobile

- [x] Project set up
- [ ] Files upload
