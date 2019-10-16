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

import { Component, EventEmitter, Input, Output } from '@angular/core';

@Component({
  selector: 'bio-visibility-selector',
  template: require('./visibility-selector.component.html')
})
export class VisibilitySelectorComponent {

  @Input() setting: string = 'public';
  @Output() changed: EventEmitter<string> = new EventEmitter<string>();

  change() {
    this.changed.emit(this.setting);
  }
}
