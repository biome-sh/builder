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

import { TestBed, ComponentFixture } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { DebugElement } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';
import { MockComponent } from 'ng2-mock-component';
import { AppStore } from '../../app.store';
import { Package } from '../../records/Package';
import * as actions from '../../actions/index';
import { PackageVersionsComponent } from './package-versions.component';

class MockAppStore {

  getState() {
    return {
      packages: {
        current: Package()
      },
      app: {
        name: 'Biome'
      }
    };
  }

  dispatch() { }
}

class MockRoute {
  parent = {
    params: of({
      origin: 'core',
      name: 'nginx'
    })
  };
}

describe('PackageVersionsComponent', () => {
  let fixture: ComponentFixture<PackageVersionsComponent>;
  let component: PackageVersionsComponent;
  let element: DebugElement;
  let store: MockAppStore;

  beforeEach(() => {

    store = new MockAppStore();
    spyOn(store, 'dispatch');
    spyOn(actions, 'fetchPackageVersions');

    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule
      ],
      declarations: [
        PackageVersionsComponent,
        MockComponent({ selector: 'bio-icon', inputs: ['symbol', 'title'] }),
        MockComponent({ selector: 'bio-channels', inputs: ['channels', 'canDemote'] }),
        MockComponent({ selector: 'bio-package-promote', inputs: ['origin', 'name', 'version', 'release', 'target', 'channel'] }),
        MockComponent({ selector: 'bio-copyable', inputs: ['text', 'style'] })
      ],
      providers: [
        { provide: AppStore, useValue: store },
        { provide: ActivatedRoute, useClass: MockRoute }
      ]
    });

    fixture = TestBed.createComponent(PackageVersionsComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
  });
});
