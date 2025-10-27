import { Component, ElementRef, OnDestroy, OnInit, signal, ViewChild } from '@angular/core';
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { ImageThumbnail } from "./image-thumbnail/image-thumbnail";
import { type Image } from "./image";
import { ImagePreviewDialog } from "./image-preview-dialog/image-preview-dialog";

@Component({
    selector: 'app-gallery',
    imports: [
        ImageThumbnail,
        ImagePreviewDialog
    ],
    templateUrl: './gallery.html',
    styleUrl: './gallery.css',
})
export class Gallery implements OnInit, OnDestroy {

    @ViewChild(ImagePreviewDialog, {static: true})
    private previewDialog!: ImagePreviewDialog;
    protected images = signal<Image[]>([]);
    private removeIndexUpdatedListener: UnlistenFn | null = null;

    ngOnInit(): void {
        this.getImages();
        this.setupListeners();
    }

    ngOnDestroy(): void {
        this.removeIndexUpdatedListener?.();
    }

    syncImages() {
        invoke("sync_images");
    }

    openPreview(index: number) {
        this.previewDialog.open(index);
    }

    private async getImages() {
        this.images.set(await invoke("get_images"));
    }

    private async setupListeners() {
        this.removeIndexUpdatedListener = await listen("index-updated", () => this.getImages());
    }
}
