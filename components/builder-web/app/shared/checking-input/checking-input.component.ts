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

import { FormControl, FormGroup, Validators } from '@angular/forms';
import { Component, Input, OnChanges, OnInit } from '@angular/core';
import { AsyncValidator } from '../../async-validator';

@Component({
  selector: 'bio-checking-input',
  template: require('./checking-input.component.html')
})
export class CheckingInputComponent implements OnInit, OnChanges {
  @Input() autofocus;
  @Input() availableMessage;
  @Input() displayName;
  @Input() form: FormGroup;
  @Input() id;
  @Input() isAvailable: Function;
  @Input() maxLength;
  @Input() name: string;
  @Input() notAvailableMessage: string;
  @Input() unmatchedMessage: string;
  @Input() pattern;
  @Input() placeholder;
  @Input() value: string;
  @Input() disabled = false;

  control: FormControl;

  private defaultMaxLength = 255;
  private defaultPattern = '^[a-z0-9][a-z0-9_-]*$';

  public ngOnInit() {
    let validators = [
      Validators.required,
      this.patternValidator.bind(this),
    ];

    const asyncValidators = [
      AsyncValidator.debounce(this.takenValidator.bind(this))
    ];

    // If explicitly passed false, don't validate for max length. If one
    // wasn't passed, use the default.
    if (this.maxLength !== false) {
      this.maxLength = this.maxLength || this.defaultMaxLength;
      validators.push(Validators.maxLength(this.maxLength));
    }

    // If explicitly passed false, don't use a pattern. If one wasn't
    // passed, use the default.
    if (this.pattern !== false) {
      this.pattern = this.pattern || this.defaultPattern;
    }

    this.notAvailableMessage = this.notAvailableMessage || 'is already in use';
    this.availableMessage = this.availableMessage || 'is available';
    this.unmatchedMessage = this.unmatchedMessage || 'must match the correct format';

    this.control = new FormControl(
      this.value,
      Validators.compose(validators),
      Validators.composeAsync(asyncValidators)
    );

    this.setDisabledState(this.control);

    this.form.addControl(this.name, this.control);
  }

  ngOnChanges() {
    if (this.control) {
      this.setDisabledState(this.control);
    }
  }

  setDisabledState(control) {
    this.disabled ? control.disable() : control.enable();
  }

  symbolForState(control) {
    if (control.pending) {
      return 'loading';
    }

    if (control.dirty && control.invalid) {
      return 'no';
    }

    if (control.valid) {
      return 'check';
    }
  }

  private patternValidator(control) {
    const value = control.value;

    if (!this.pattern || !value || value.match(this.pattern)) {
      return null;
    } else {
      return { invalidFormat: true };
    }
  }

  private takenValidator(control) {
    return new Promise(resolve => {
      // If we're empty or invalid, don't attempt to validate.
      if ((control.errors && control.errors.required) ||
          (control.errors && control.errors.invalidFormat) ||
          control.disabled) {
        resolve(null);
      }

      if (this.isAvailable) {
        this.isAvailable(control.value).
          then(() => resolve(null)).
          catch(() => resolve({ taken: true }));
      } else {
        resolve(null);
      }
    });
  }
}
