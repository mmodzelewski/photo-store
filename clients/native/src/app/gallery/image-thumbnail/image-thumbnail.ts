import { Component, computed, input, output } from '@angular/core';
import { type Image, toCoverUri } from "../image";

@Component({
    selector: 'app-image-thumbnail',
    imports: [],
    templateUrl: './image-thumbnail.html',
    styleUrl: './image-thumbnail.scss',
})
export class ImageThumbnail {

    image = input.required<Image>();
    protected convertedPath = computed(() => {
        const image = this.image();
        return toCoverUri(image);
    });

}
