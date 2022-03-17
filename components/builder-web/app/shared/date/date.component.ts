// Biome project based on Chef Habitat's code (c) 2016-2022 Chef Software, Inc
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

import { Component, Input } from '@angular/core';

import * as moment from 'moment';

@Component({
  selector: 'bio-date',
  template: `<mat-label [matTooltip]="tooltip" matTooltipPosition="above">{{ displayDate }}</mat-label>`
})
export class DateComponent {
  @Input() date: string = '';

  get tooltip() {
    let tip;

    if (this.date && this.date.trim() !== '') {
      tip = this.date;
    }

    return moment(this.date).format('ddd DD-MMM-YYYY, hh:mm A');
  }

  get displayDate() {
    return moment.utc(this.date).fromNow();
  }
}
