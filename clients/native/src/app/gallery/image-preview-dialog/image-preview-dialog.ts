import { Component, computed, ElementRef, signal, ViewChild } from '@angular/core';
import { input } from '@angular/core';
import { type Image, toContainUri } from "../image";

@Component({
    selector: 'app-image-preview-dialog',
    imports: [],
    templateUrl: './image-preview-dialog.html',
    styleUrl: './image-preview-dialog.scss',
})
export class ImagePreviewDialog {

    images = input.required<Image[]>();

    @ViewChild('dialog', {static: true})
    private dialog!: ElementRef<HTMLDialogElement>;

    protected index = signal(0);
    protected imagePath = computed(() => {
        const image = this.images()[this.index()];
        return toContainUri(image);
    })

    open(imageIndex: number) {
        this.dialog.nativeElement.showModal();
        this.index.set(imageIndex);
    }

    prev() {
        const index = this.index();
        if (index > 0) {
            this.index.set(index - 1);
        }
    }

    hasPrevious() {
        return this.index() > 0;
    }

    next() {
        const index = this.index();
        if (index < this.images().length - 1) {
            this.index.set(index + 1);
        }
    }

    hasNext() {
        return this.index() < this.images().length - 1;
    }

}
