import { Component, computed, ElementRef, inject, OnDestroy, OnInit, signal, ViewChild } from '@angular/core';
import { pictureDir } from "@tauri-apps/api/path";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Router } from "@angular/router";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { interval } from "rxjs";
import { toSignal } from "@angular/core/rxjs-interop";
import { NgOptimizedImage } from "@angular/common";

interface ThumbnailsGeneratedPayload {
    done: number;
    total: number;
    latest: string;
}

interface FilesIndexedPayload {
    total: number;
}

@Component({
    selector: 'app-intro',
    imports: [],
    templateUrl: './intro.html',
    styleUrl: './intro.scss',
})
export class Intro implements OnInit, OnDestroy {

    @ViewChild('dialog', {static: true})
    private dialog!: ElementRef<HTMLDialogElement>;
    private router = inject(Router);
    private removeListener: UnlistenFn[] = [];
    protected thumbnailsGenerated = signal<ThumbnailsGeneratedPayload | null>(null);
    private latestThumbnail = computed(() => {
        const thumbnails = this.thumbnailsGenerated();
        if (thumbnails) {
            return convertFileSrc(thumbnails.latest);
        }
        return "";
    });
    private intervalObservable = interval(2000);
    private intervalSignal = toSignal(this.intervalObservable);
    protected thumbnailToDisplay = computed(() => {
        const _interval = this.intervalSignal();
        return this.latestThumbnail();
    })
    protected filesIndexed = signal<number | null>(null);

    ngOnInit(): void {
        this.setupListeners();
    }

    ngOnDestroy(): void {
        this.removeListener.forEach(unlisten => unlisten());
    }

    async selectDir() {
        const picsDir = await pictureDir();
        const dirs = await open({
            defaultPath: picsDir,
            directory: true,
            recursive: true,
            multiple: true,
        });
        if (dirs) {
            await invoke("save_images_dirs", {dirs});
            this.router.navigateByUrl("/gallery");
        }
    }

    private async setupListeners() {
        const unlisten1 = await listen<ThumbnailsGeneratedPayload>(
            "thumbnails-generated",
            (event) => {
                console.log('received thumbnails generated');
                this.thumbnailsGenerated.set(event.payload);
            },
        );
        this.removeListener.push(unlisten1);

        const unlisten2 = await listen<FilesIndexedPayload>(
            "files-indexed",
            (event) => {
                console.log('received files indexed');
                this.filesIndexed.set(event.payload.total);
                this.dialog.nativeElement.showModal();
            },
        );
        this.removeListener.push(unlisten2);

    }
}
