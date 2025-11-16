import { Component, ElementRef, inject, OnDestroy, OnInit, ViewChild } from '@angular/core';
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FormsModule } from "@angular/forms";
import { Router } from "@angular/router";

@Component({
    selector: 'app-login',
    imports: [
        FormsModule
    ],
    templateUrl: './login.html',
    styleUrl: './login.scss',
})
export class Login implements OnInit, OnDestroy {

    protected passphrase = "";
    @ViewChild('dialog', {static: true})
    private dialog!: ElementRef<HTMLDialogElement>;
    private removeAuthenticatedListener: (() => void) | null = null;
    private router = inject(Router);


    login() {
        console.log('login clicked');
        invoke("authenticate")
    }

    async initPrivateKey() {
        try {
            await invoke("get_private_key", {passphrase: this.passphrase});
            this.dialog.nativeElement.close();
            this.router.navigateByUrl("/intro");
        } catch (e) {
            console.log(e);
        }
    }

    ngOnInit(): void {
        this.setUpAuthenticatedListener();
    }

    ngOnDestroy(): void {
        this.removeAuthenticatedListener?.();
    }

    private async setUpAuthenticatedListener() {
        console.log('auth listener set up')
        this.removeAuthenticatedListener = await listen<void>("authenticated", () => {
            console.log('authenticated')
            this.dialog.nativeElement.showModal();
        });
    }

}
