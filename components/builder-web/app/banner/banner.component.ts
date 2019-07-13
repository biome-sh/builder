import { Component } from '@angular/core';
import { AppStore } from '../app.store';

@Component({
  selector: 'bio-banner',
  template: require('./banner.component.html')
})
export class BannerComponent {
  dismissed: boolean = false;

  constructor(private store: AppStore) {}

  get hidden() {
    return this.profile.id && !this.dismissed;
  }

  get profile() {
    return this.store.getState().users.current.profile;
  }

  dismiss() {
    this.dismissed = true;
  }
}
