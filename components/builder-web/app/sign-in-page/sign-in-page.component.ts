// Community fork of Chef Habitat
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { Component, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { AppStore } from '../app.store';
import { setLayout, signOut } from '../actions/index';
import config from '../config';

@Component({
  template: require('./sign-in-page.component.html')
})
export class SignInPageComponent implements OnDestroy {

  constructor(private store: AppStore, private title: Title) {
    store.dispatch(signOut(false));
    this.title.setTitle(`Sign In | ${store.getState().app.name}`);
    this.store.dispatch(setLayout('sign-in'));
  }

  get providerType() {
    return this.store.getState().oauth.provider.type;
  }

  get providerName() {
    return this.store.getState().oauth.provider.name;
  }

  get loginUrl() {
    const provider = this.store.getState().oauth.provider;

    const qs = Object.keys(provider.params)
      .map(k => `${k}=${encodeURIComponent(provider.params[k])}`)
      .join('&');

    return `${provider.authorizeUrl}?${qs}`;
  }

  get signupUrl() {
    return this.store.getState().oauth.provider.signupUrl;
  }

  get wwwUrl() {
    return config['www_url'];
  }

  ngOnDestroy() {
    this.store.dispatch(setLayout('default'));
  }
}
