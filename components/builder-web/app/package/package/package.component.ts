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

import { Component, OnInit, OnDestroy } from '@angular/core';
import { combineLatest, Subject, Subscription } from 'rxjs';
import { filter, map, takeUntil } from 'rxjs/operators';
import { List } from 'immutable';
import { PackageJobComponent } from '../package-job/package-job.component';
import { PackageJobsComponent } from '../package-jobs/package-jobs.component';
import { PackageLatestComponent } from '../package-latest/package-latest.component';
import { PackageReleaseComponent } from '../package-release/package-release.component';
import { PackageVersionsComponent } from '../package-versions/package-versions.component';
import { AppStore } from '../../app.store';
import {
  fetchJobs, fetchIntegrations, fetchLatestPackage, fetchLatestInChannel, fetchOrigin, fetchProject,
  fetchPackageSettings, fetchPackageVersions, setCurrentPackageTarget, clearPackageVersions, fetchPackage, fetchPackageChannels
} from '../../actions/index';
import { targetFrom, targets as allPlatforms, latestBase } from '../../util';
import { fetchOriginChannels } from '../../actions/origins';

@Component({
  template: require('./package.component.html')
})
export class PackageComponent implements OnInit, OnDestroy {
  origin: string;
  name: string;
  target: string;
  version: string;
  release: string;
  isOriginMember: boolean = false;
  showSidebar: boolean = false;
  showReleaseSidebar: boolean = false;
  showActiveJob: boolean = false;
  useFullWidth: boolean = true;

  private isDestroyed$: Subject<boolean> = new Subject();
  private poll: number;

  constructor(private store: AppStore) {
    const origin$ = this.store.observe('router.route.params.origin').pipe(filter(v => v));
    const name$ = this.store.observe('router.route.params.name').pipe(filter(v => v));
    const target$ = this.store.observe('router.route.params.target');
    const version$ = this.store.observe('router.route.params.version').pipe(filter(v => v));
    const release$ = this.store.observe('router.route.params.release').pipe(filter(v => v));
    const token$ = this.store.observe('session.token');
    const origins$ = this.store.observe('origins.mine');
    const originsCurrent$ = this.store.observe('origins.current');
    const originsCurrentChannels$ = this.store.observe('origins.current.channels');
    const platforms$ = this.store.observe('packages.currentPlatforms');
    const versionsLoading$ = this.store.observe('packages.ui.versions.loading');
    const isOriginMember$ = combineLatest(origin$, origins$)
      .pipe(map(([origin, origins]) => !!origins.find(o => o.name === origin)));

    this.store.observe('router.route.params')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(({origin, name, target, version, release}) => {
        this.origin = origin;
        this.name = name;
        this.target = target;
        this.version = version;
        this.release = release;
      });

    isOriginMember$
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(isOriginMember => {
        this.isOriginMember = isOriginMember;
      });

    origin$
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(() => this.fetchOrigin());

    originsCurrent$
      .pipe(
        takeUntil(this.isDestroyed$),
        filter((origin) => origin.name !== undefined)
      )
      .subscribe((origin) => {
        this.store.dispatch(fetchOriginChannels(origin.name));
      });

    combineLatest(origin$, name$, isOriginMember$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(([origin, name, isOriginMember]) => {
        this.fetchPackageVersions();
        if (isOriginMember) {
          this.fetchPackageSettings();
        }
      });

    combineLatest(origin$, name$, version$, release$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(() => this.fetchRelease());

    combineLatest(origin$, name$, token$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(() => this.fetchJobs());

    combineLatest(versionsLoading$, origin$, name$, target$, platforms$)
      .pipe(
        takeUntil(this.isDestroyed$),
        filter(([versionsLoading]) => !versionsLoading)
      )
      .subscribe(([versionsLoading, origin, name, target, platforms]) => {
        const latestVersion = this.store.getState().packages.versions?.[0];
        const tgt = target ? targetFrom('param', target).id : latestVersion ? latestVersion.platforms[0] : allPlatforms[0].id;
        const currentTarget = targetFrom('id', tgt);
        this.target = currentTarget.id;
        this.store.dispatch(setCurrentPackageTarget(currentTarget));

        // This particularly happens when we refresh the page
        // Just ensure we are on the latest page
        if (!this.ident.version && !this.ident.release) {
          this.fetchLatest();
          if (target === undefined) {
            this.fetchLatestStable('');
          }
          else {
            this.fetchLatestStable(this.target);
          }


          // This check whether channel is exist in current origin
          if (target === undefined) {
            this.fetchCurrentLts('');
          }
          else {
            this.fetchCurrentLts(this.target);
          }
        }
      });

    combineLatest(versionsLoading$, origin$, name$, target$, token$, origins$, platforms$)
      .pipe(
        takeUntil(this.isDestroyed$),
        filter(([versionsLoading]) => !versionsLoading)
      )
      .subscribe(() => this.fetchProject());
  }

  ngOnInit() {
    // When a build is active, check on it periodically so we can
    // indicate when it completes.
    this.poll = window.setInterval(() => {
      if (this.building) {
        this.fetchJobs();
      }
    }, 10000);
  }

  // Check if the given channel exist in current origin
  isChannelExistInOrigin(channelName) {
    const channelExist = this.store.getState().origins.current.channels.find((channel) => {
      return channel.name === channelName;
    });

    return channelExist?.name === channelName ? true : false;
  }

  ngOnDestroy() {
    window.clearInterval(this.poll);

    this.store.dispatch(setCurrentPackageTarget(undefined));
    this.store.dispatch(clearPackageVersions());

    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get subheading() {
    return this.activeRelease ? 'artifact' : 'package';
  }

  get isStandardPkg() {
    // A package can be standard or native. That means all package versions should be of the same type.
    // It is sufficient to check the type on the latest and fetched on the initial request.
    const _type = this.store.getState().packages.latest.package_type || 'standard';
    return _type.toLowerCase() === 'standard';
  }

  get nonStandardLabel() {
    return 'native';
  }

  get ident() {
    return {
      origin: this.origin,
      name: this.name,
      version: this.version,
      release: this.release
    };
  }

  get isNewProject() {
    return this.store.getState().packages.currentPlatforms.length === 0;
  }

  get hasPlan() {
    return this.store.getState().projects.currentProjects.length > 0;
  }

  get builderEnabled() {
    return this.store.getState().features.builder;
  }

  get visibilityEnabled() {
    return this.store.getState().features.visibility;
  }

  get activePackage() {
    return this.store.getState().packages.current;
  }

  get activeSettings() {
    return this.store.getState().packages.currentSettings;
  }

  get activeRelease() {
    return this.version && this.release ? this.activePackage : null;
  }

  get activeJobs(): List<any> {
    const activeStates = ['Dispatched', 'Pending', 'Processing'];

    return this.store.getState().jobs.visible.filter((b) => {
      return activeStates.indexOf(b.state.toString()) !== -1;
    });
  }

  get activeJob() {
    let active = this.activeJobs.last();
    return active;
  }

  get building(): boolean {
    return this.activeJobs.size > 0;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get visibility() {
    return this.store.getState().projects.current.visibility;
  }

  get defaultVisibility() {
    return this.store.getState().origins.current.default_package_visibility;
  }

  onRouteActivate(routedComponent) {
    this.showSidebar = false;
    this.showReleaseSidebar = false;
    this.showActiveJob = false;

    [
      PackageJobsComponent,
      PackageLatestComponent,
      PackageVersionsComponent
    ].forEach((c) => {
      if (routedComponent instanceof c) {
        this.showSidebar = true;
        this.showActiveJob = true;
      }
    });

    if (routedComponent instanceof PackageJobComponent) {
      this.useFullWidth = true;
    }

    if (routedComponent instanceof PackageReleaseComponent) {
      this.showReleaseSidebar = true;
    }
  }

  private fetchOrigin() {
    this.store.dispatch(fetchOrigin(this.origin));
  }

  private getLatestPlatform(target) {
    if (target === undefined) {
      target = 'x86_64-linux';
    }

    const versions = this.store.getState().packages?.versions;
    if (!versions) {
      return target;
    }

    const platformFound = versions.some(item => item.platforms.includes(target));
    if (platformFound) {
      return target;
    }

    return versions[0]?.platforms[0];
  }

  private fetchLatest() {
    const target = this.getLatestPlatform(this.target);
    this.store.dispatch(fetchLatestPackage(this.origin, this.name, target));
    const currentTarget = targetFrom('id', target);
    this.store.dispatch(setCurrentPackageTarget(currentTarget));
  }

  private fetchLatestStable(target: string) {
    this.store.dispatch(fetchLatestInChannel(this.origin, this.name, 'stable', target));
  }

  private fetchCurrentLts(target: string) {
    this.store.dispatch(fetchLatestInChannel(this.origin, this.name, latestBase, target));
}

  private fetchPackageSettings() {
    this.store.dispatch(fetchPackageSettings(this.origin, this.name, this.token));
  }

  private fetchPackageVersions() {
    this.store.dispatch(fetchPackageVersions(this.origin, this.name));
  }

  private fetchProject() {
    if (this.token && this.origin && this.name && this.target && this.isOriginMember) {
      this.store.dispatch(fetchProject(this.origin, this.name, this.target, this.token, false));
      this.store.dispatch(fetchIntegrations(this.origin, this.token));
    }
  }

  private fetchJobs() {
    if (this.token) {
      this.store.dispatch(fetchJobs(this.origin, this.name, this.token));
    }
  }

  private fetchRelease() {
    this.store.dispatch(fetchPackage({ ident: this.ident }));
  }
}
