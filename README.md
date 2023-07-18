# Photo store

Photo store is a comprehensive media storage application designed to provide
a seamless experience for managing, sharing, and syncing your media files
across multiple platforms.

### Todo

- [ ] Initial user auth implementation
- [ ] Refactor sending files to R2/S3
- [ ] Cargo workspace for sharing code between apps?
- [ ] API design

#### Desktop

- [x] Local images indexing
- [x] Thumbnails generation for images
- [x] Pagination for images loading
- [ ] ~~Add [Persisted Scope plugin](https://github.com/tauri-apps/plugins-workspace/tree/v1/plugins/persisted-scope)
      once the [PR](https://github.com/tauri-apps/plugins-workspace/pull/32) is merged~~ -
      no longer needed, it's covered by saving and restoring settings from db
- [x] Change command for indexing images to async
- [ ] Add exif data to images index (original date added)
- [x] Display images in chronological order
- [ ] Change process for generating thumbnails - maybe on demand?
- [ ] Add custom assets protocol for getting images by id and resolution
- [ ] Test performance for storing thumbnails in sqlite
- [ ] Add option for adding/removing source directories
- [ ] Watch file changes: add, remove, move
- [x] Add prev/next navigation when previwing an image
- [ ] Generate multiple thumbnails sizes
