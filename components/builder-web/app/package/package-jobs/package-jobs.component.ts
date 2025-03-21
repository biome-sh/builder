// Biome project based on Chef Habitat's code (c) 2016-2020 Chef Software, Inc
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
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';
import { AppStore } from '../../app.store';

@Component({
  template: require('./package-jobs.component.html')
})
export class PackageJobsComponent implements OnDestroy {
  origin: string;
  name: string;

  private sub: Subscription;

  constructor(
    private route: ActivatedRoute,
    private store: AppStore,
    private router: Router,
    private title: Title
  ) {

    this.sub = this.route.parent.params.subscribe((params) => {
      this.origin = params['origin'];
      this.name = params['name'];
      this.title.setTitle(`Packages › ${this.origin}/${this.name} › Build Jobs | ${store.getState().app.name}`);
    });
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get jobs() {
    return this.store.getState().jobs.visible;
  }

  onSelect(job) {
    this.router.navigate(['pkgs', this.origin, this.name, 'jobs', job.id]);
  }
}
