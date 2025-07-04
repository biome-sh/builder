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

import { Component, Inject } from '@angular/core';
import { MatDialogRef, MAT_DIALOG_DATA } from '@angular/material';

@Component({
  template: require('./promote-confirm.dialog.html')
})
export class PromoteConfirmDialog {
  selectedChannel: any = undefined;

  constructor(
    private ref: MatDialogRef<PromoteConfirmDialog>,
    @Inject(MAT_DIALOG_DATA) private data: any
  ) { }

  get channelList() {
    return this.data.channelList;
  }

  get heading() {
    return this.data.heading || 'Confirm';
  }

  get action() {
    return this.data.action || 'do it';
  }

  ok() {
    this.ref.close({confirmed: true, selectedChannel : this.selectedChannel?.name});
  }

  cancel() {
    this.ref.close({confirmed : false});
  }

}
