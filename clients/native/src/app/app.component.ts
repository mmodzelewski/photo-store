import { Component, inject, OnInit } from "@angular/core";
import { Router, RouterOutlet } from "@angular/router";
import { invoke } from "@tauri-apps/api/core";

@Component({
    selector: "app-root",
    imports: [RouterOutlet],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent implements OnInit {

    private router = inject(Router);

    ngOnInit(): void {
        this.redirect();
    }

    private async redirect() {
        const status = await invoke("get_status");
        switch (status) {
            case 'directories_selected':
                this.router.navigateByUrl("/gallery");
                break;
            case 'after_login':
                this.router.navigateByUrl("/intro");
                break;
        }
    }

}
