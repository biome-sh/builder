import { Component, Input } from '@angular/core';
import { targetFrom } from '../../util';

@Component({
  selector: 'bio-platform-icon',
  template: `<bio-icon *ngIf="target" [symbol]="target.param" class="icon-os" [title]="target.title"></bio-icon>`
})
export class PlatformIconComponent {

  @Input() platform;

  get target() {
    return targetFrom('id', this.platform);
  }
}
