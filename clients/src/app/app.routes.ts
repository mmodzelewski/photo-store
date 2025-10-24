import { Routes } from "@angular/router";

export const routes: Routes = [
    {
        path: "login",
        loadComponent: () => import("./login/login").then(m => m.Login)
    },
    {
        path: "intro",
        loadComponent: () => import("./intro/intro").then(m => m.Intro)
    },
    {
        path: "gallery",
        loadComponent: () => import("./gallery/gallery").then(m => m.Gallery)
    },
    {
        path: "**",
        redirectTo: "login"
    }
];
