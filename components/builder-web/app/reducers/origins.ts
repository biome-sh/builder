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

import { List } from 'immutable';
import { sortBy } from 'lodash';
import * as actionTypes from '../actions/index';
import initialState from '../initial-state';
import { Origin } from '../records/Origin';

export default function origins(state = initialState['origins'], action) {
  switch (action.type) {

    case actionTypes.CLEAR_MY_ORIGINS:
      return state.setIn(['mine'], List())
        .setIn(['ui', 'mine', 'errorMessage'], undefined)
        .setIn(['ui', 'mine', 'loading'], true);

    case actionTypes.CLEAR_MY_ORIGIN_INVITATIONS:
      return state.setIn(['myInvitations'], List());

    case actionTypes.CLEAR_INTEGRATION:
      return state.setIn(['currentIntegrations', 'selected'], undefined);

    case actionTypes.CLEAR_INTEGRATIONS:
      return state.setIn(['currentIntegrations', 'selected'], undefined).
        setIn(['currentIntegrations', 'integrations'], {});

    case actionTypes.POPULATE_MY_ORIGINS:
      if (action.error) {
        return state.setIn(['mine'], List()).
          setIn(['ui', 'mine', 'errorMessage'], action.error.message).
          setIn(['ui', 'mine', 'loading'], false);
      } else {
        return state.setIn(['mine'], List(action.payload.map(origin =>
          Origin({
            name: origin.name,
            package_count: origin.package_count,
            default_package_visibility: origin.default_package_visibility
          })
        )))
        .setIn(['ui', 'mine', 'errorMessage'], undefined)
        .setIn(['ui', 'mine', 'loading'], false);
      }

    case actionTypes.POPULATE_MY_ORIGIN_INVITATIONS:
      return state.setIn(['myInvitations'],
        List(action.payload));

    case actionTypes.POPULATE_ORIGIN_INVITATIONS:
      return state.setIn(['currentPendingInvitations'],
        List(action.payload));

    case actionTypes.POPULATE_ORIGIN_MEMBERS:
      return state.setIn(['currentMembers'],
        List(action.payload));

    case actionTypes.POPULATE_ORIGIN_INTEGRATION:
      if (action.payload) {
        return state.setIn(['currentIntegrations', 'selected'], action.payload);
      } else {
        return state.setIn(['currentIntegrations', 'selected'], undefined);
      }

    case actionTypes.POPULATE_ORIGIN_INTEGRATIONS:
      if (action.payload) {
        return state.setIn(['currentIntegrations', 'integrations'], action.payload);
      } else {
        return state.setIn(['currentIntegrations', 'integrations'], {});
      }

    case actionTypes.POPULATE_ORIGIN_PUBLIC_KEYS:
      if (action.error) {
        return state.setIn(
          ['ui', 'current', 'publicKeyListErrorMessage'],
          action.error.message
        );
      } else {
        return state.setIn(['currentPublicKeys'], List(action.payload)).
          setIn(
            ['ui', 'current', 'publicKeyListErrorMessage'],
            undefined
          );
      }

    case actionTypes.POPULATE_ORIGIN_SECRETS:
      if (action.payload) {
        return state.set('currentSecrets', List(
          sortBy(action.payload, 'name')
            .map(secret => { return { key: secret.name, value: secret.value }; }))
        );
      } else {
        return state.set('currentSecrets', List());
      }

    case actionTypes.POPULATE_ORIGIN_CHANNELS:
      if (action.payload) {
        return state.setIn(['current', 'channels'], action.payload);
      } else {
        return state.set('channels', List());
      }

    case actionTypes.SET_CURRENT_ORIGIN:
      if (action.error) {
        return state.set('current', Origin()).
          setIn(['ui', 'current', 'errorMessage'],
            action.error.message).
          setIn(['ui', 'current', 'loading'], false).
          setIn(['ui', 'current', 'exists'], false);
      } else {
        return state.set('current', Origin(action.payload)).
          setIn(['ui', 'current', 'errorMessage'], undefined).
          setIn(['ui', 'current', 'exists'], true).
          setIn(['ui', 'current', 'loading'], false);
      }
    case actionTypes.SET_CURRENT_ORIGIN_CREATING_FLAG:
      return state.setIn(['ui', 'current', 'creating'], action.payload);

    case actionTypes.SET_CURRENT_ORIGIN_ADDING_PRIVATE_KEY:
      return state.setIn(['ui', 'current', 'addingPrivateKey'],
        action.payload);

    case actionTypes.SET_CURRENT_ORIGIN_ADDING_PUBLIC_KEY:
      return state.setIn(['ui', 'current', 'addingPublicKey'],
        action.payload);

    case actionTypes.SET_CURRENT_ORIGIN_LOADING:
      return state.setIn(['ui', 'current', 'loading'],
        action.payload);

    case actionTypes.SET_INTEGRATION_CREDS_VALIDATION:
      return state.setIn(['currentIntegrations', 'ui', 'creds'], action.payload);

    case actionTypes.SET_ORIGIN_INTEGRATION_SAVE_ERROR_MESSAGE:
      return state.setIn(['ui', 'current', 'integrationsSaveErrorMessage'],
        action.payload);

    case actionTypes.SET_ORIGIN_USER_INVITE_ERROR_MESSAGE:
      return state.setIn(['ui', 'current', 'userInviteErrorMessage'],
        action.payload);

    case actionTypes.TOGGLE_ORIGIN_PICKER:
      return state.setIn(['ui', 'isPickerOpen'],
        !state.getIn(['ui', 'isPickerOpen']));

    default:
      return state;
  }
}
