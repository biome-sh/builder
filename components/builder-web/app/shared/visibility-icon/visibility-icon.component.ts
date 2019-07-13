import { Component, Input } from '@angular/core';

@Component({
  selector: 'bio-visibility-icon',
  template: `<bio-icon [symbol]="symbol" class="icon-visibility" [title]="title"></bio-icon>`
})
export class VisibilityIconComponent {

  @Input() visibility: string;
  @Input() prefix: string;

  get symbol() {
    return this.visibility === 'public' ? 'public' : 'lock';
  }

  get title() {
    const t = this.visibility === 'public' ? 'Public' : 'Private';
    return this.prefix ? `${this.prefix} ${t}` : t;
  }
}
