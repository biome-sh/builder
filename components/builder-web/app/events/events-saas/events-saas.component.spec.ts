// Biome project based on Chef Habitat's code (c) 2016-2021 Chef Software, Inc
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

import { DebugElement } from '@angular/core';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { ReactiveFormsModule } from '@angular/forms';
import { MatInputModule } from '@angular/material';
import { By } from '@angular/platform-browser';
import { NoopAnimationsModule } from '@angular/platform-browser/animations';
import { ActivatedRoute } from '@angular/router';
import { RouterTestingModule } from '@angular/router/testing';
import { of } from 'rxjs';
import { List } from 'immutable';
import { MockComponent } from 'ng2-mock-component';

import { AppStore } from '../../app.store';
import { EventsSaaSComponent } from './events-saas.component';

class MockAppStore {
  static state;

  getState() {
    return MockAppStore.state;
  }

  dispatch() { }
}

class MockRoute {
  get params() {
    return of({});
  }
}

describe('EventsSaaSComponent', () => {
  let fixture: ComponentFixture<EventsSaaSComponent>;
  let component: EventsSaaSComponent;
  let element: DebugElement;
  let store: AppStore;

  beforeEach(() => {
    MockAppStore.state = {
      events: {
        visible: List(),
        ui: {
          visible: {}
        }
      },
      app: {
        name: 'Biome'
      }
    };
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [
        ReactiveFormsModule,
        RouterTestingModule,
        MatInputModule,
        NoopAnimationsModule
      ],
      declarations: [
        MockComponent({
          selector: 'bio-event-results',
          inputs: ['errorMessage', 'noEvents', 'events']
        }),
        EventsSaaSComponent
      ],
      providers: [
        { provide: AppStore, useClass: MockAppStore },
        { provide: ActivatedRoute, useClass: MockRoute }
      ]
    });

    fixture = TestBed.createComponent(EventsSaaSComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    store = TestBed.get(AppStore);
  });

  describe('given the events', () => {

    beforeEach(() => {
      fixture.detectChanges();
    });

    it('shows the Builder Events heading', () => {
      let heading = element.query(By.css('.events-component h1'));
      expect(heading.nativeElement.textContent).toBe('Builder Events (SaaS)');
    });
  });
});
