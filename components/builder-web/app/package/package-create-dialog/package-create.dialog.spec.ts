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

import { DebugElement } from '@angular/core';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { By } from '@angular/platform-browser';
import { MatDialog, MatDialogRef } from '@angular/material';
import { MockComponent } from 'ng2-mock-component';
import { AppStore } from '../../app.store';
import { PackageCreateDialog } from '../package-create-dialog/package-create.dialog';
import * as actions from '../../actions';

class MockAppStore {
  getState() {
    return {
      session: {
        token: 'token'
      },
      origins: {
        current: {
          name: 'mock-origin'
        }
      }
    };
  }

  dispatch() { }
}

class MockDialog { }

describe('PackageCreateDialog', () => {
  let fixture: ComponentFixture<PackageCreateDialog>;
  let component: PackageCreateDialog;
  let element: DebugElement;
  let store: MockAppStore;

  beforeEach(() => {

    store = new MockAppStore();
    spyOn(store, 'dispatch');
    spyOn(actions, 'createEmptyPackage');

    TestBed.configureTestingModule({
      imports: [
        FormsModule,
        ReactiveFormsModule,
        RouterTestingModule
      ],
      declarations: [
        PackageCreateDialog,
        MockComponent({ selector: 'bio-checking-input', inputs: ['form', 'isAvailable']})
      ],
      providers: [
        { provide: AppStore, useValue: store },
        { provide: MatDialog, useClass: MockDialog },
        { provide: MatDialogRef, useValue: {} }
      ]
    });

    fixture = TestBed.createComponent(PackageCreateDialog);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('when the form submits', () => {

    it('should call createPackage()', () => {
      spyOn(component, 'onSubmit').and.callThrough();

      component.createPackageForm.value.name = 'testPackageName';
      const submitButton = element.query(By.css('#package-submit-button'));
      submitButton.nativeElement.click();
      fixture.detectChanges();

      expect(component.onSubmit).toHaveBeenCalled();
      expect(actions.createEmptyPackage).toHaveBeenCalled();
    });
  });
});