# Photo store

Photo store is a comprehensive media storage application designed to provide 
a seamless experience for managing, sharing, and syncing your media files 
across multiple platforms.

### Todo

- [ ] Initial user auth implementation
- [ ] Refactor sending files to R2/S3
- [x] Local images indexing
- [x] Thumbnails generation for images
- [x] Pagination for images loading
- [ ] Cargo workspace for sharing code between apps?
- [ ] API design
- [ ] Add [Persisted Scope plugin](https://github.com/tauri-apps/plugins-workspace/tree/v1/plugins/persisted-scope) 
once the [PR](https://github.com/tauri-apps/plugins-workspace/pull/32) is merged
- [ ] Change command for getting images to async
- [ ] Add exif data to images index
- [ ] Display images in chronological order
- [ ] Change process for generating thumbnails - maybe on demand?
- [ ] Add custom assets protocol for getting images by id and resolution
- [ ] Test performance for storing thumbnails in sqlite
