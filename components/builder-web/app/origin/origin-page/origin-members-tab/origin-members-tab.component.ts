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
import { FormControl, FormGroup, FormBuilder, Validators } from '@angular/forms';
import { Router } from '@angular/router';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { List } from 'immutable';
import { MatDialog } from '@angular/material';
import { SimpleConfirmDialog } from '../../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { DepartOriginDialog } from './dialog/depart-origin.dialog';
import { AppStore } from '../../../app.store';
import { deleteOriginInvitation, inviteUserToOrigin } from '../../../actions/index';
import { Origin } from '../../../records/Origin';
import { deleteOriginMember, departOrigin, fetchOriginMembers, fetchOriginInvitations } from '../../../actions/index';
import config from '../../../config';

@Component({
  template: require('./origin-members-tab.component.html')
})
export class OriginMembersTabComponent implements OnInit, OnDestroy {
  form: FormGroup;
  control: FormControl;
  sub: Subscription;
  origin;

  constructor(
    formBuilder: FormBuilder,
    private route: ActivatedRoute,
    private router: Router,
    private store: AppStore,
    private confirmDialog: MatDialog,
    private departOriginDialog: MatDialog,
    private title: Title
  ) {
    this.form = formBuilder.group({});
  }

  ngOnInit() {
    this.sub = this.route.parent.params.subscribe(params => {
      this.origin = Origin({ name: params['origin'] });
      this.title.setTitle(`Origins › ${this.origin.name} › Members | ${this.store.getState().app.name}`);
      this.store.dispatch(fetchOriginMembers(this.origin.name, this.token));
      this.store.dispatch(fetchOriginInvitations(this.origin.name, this.token));
    });

    this.control = new FormControl('', Validators.required);
    this.form.addControl('username', this.control);
  }

  ngOnDestroy() {
    this.sub.unsubscribe();
  }

  get ui() {
    return this.store.getState().origins.ui.current;
  }

  get errorMessage() {
    return this.ui.userInviteErrorMessage;
  }

  get invitations(): List<Object> {
    return this.store.getState().origins.currentPendingInvitations;
  }

  get isOriginOwner() {
    return this.store.getState().users.current.profile.id === this.store.getState().origins.current.owner_id;
  }

  get members(): List<Object> {
    return this.store.getState().origins.currentMembers;
  }

  get docsUrl() {
    return config['docs_url'];
  }

  get providerName() {
    return this.store.getState().oauth.provider.name;
  }

  get token() {
    return this.store.getState().session.token;
  }

  private get isPrivate() {
    return this.store.getState().origins.current.default_package_visibility === 'private' ? true : false;
  }

  canDelete(member) {
    return this.store.getState().users.current.profile.name !== member;
  }

  delete(member) {
    const data = {
      heading: 'Confirm remove',
      body: `Are you sure you want to remove this member? Doing so will remove
                revoke access to this origin and its private packages.`,
      action: 'remove member'
    };

    this.confirm(data, () => {
      this.store.dispatch(deleteOriginMember(this.origin.name, member, this.token));
    });
  }

  rescind(invitation) {
    const data = {
      heading: 'Confirm rescind',
      body: `Are you sure you want to rescind this invitation? Doing so will remove
                access to this origin and its private packages.`,
      action: 'rescind it'
    };

    this.confirm(data, () => {
      this.store.dispatch(deleteOriginInvitation(invitation.id, this.origin.name, this.token));
    });
  }

  departFromOrigin(): void {
    const data = {
      originName: this.origin.name
    };

    this.departOriginDialog
      .open(DepartOriginDialog, { width: '480px', data: data })
      .afterClosed()
      .subscribe((confirmed) => {
        if (confirmed) {
          this.store.dispatch(departOrigin(this.origin.name, this.token, (originName) => {
            this.departureRouting(this.isPrivate, originName);
          }));
        }
      });
  }

  submit(username: string) {
    this.store.dispatch(inviteUserToOrigin(username, this.origin.name, this.token));
    const field = this.form.get('username');
    field.setValue('');
    field.markAsPristine();
  }

  private departureRouting(isPrivate: boolean, originName: string): void {
    // Check to see if the origin has default package visibility of private and route accordingly
    if (isPrivate) {
      this.router.navigateByUrl('/origins');
    } else {
      this.router.navigateByUrl('/', { skipLocationChange: true }).then(() => {
        this.router.navigate(['/origins', originName]);
      });
    }
  }

  private confirm(data, then) {
    this.confirmDialog
      .open(SimpleConfirmDialog, { width: '480px', data: data })
      .afterClosed()
      .subscribe((confirmed) => {
        if (confirmed) {
          then();
        }
      });
  }
}
