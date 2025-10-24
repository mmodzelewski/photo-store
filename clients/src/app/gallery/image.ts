export interface Image {
    id: string
}

export function toCoverUri(image: Image): string {
    return `image://localhost/${image.id}/512/cover`;
}

export function toContainUri(image: Image): string {
    return `image://localhost/${image.id}/1920/contain`;
}

export function toOriginalUri(image: Image): string {
    return `image://localhost/${image.id}/original`;
}
