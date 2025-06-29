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

import { marked } from 'marked';
import * as actionTypes from '../actions/index';
import initialState from '../initial-state';
import { Package } from '../records/Package';
import { List } from 'immutable';
import { targetsFromPkgVersions } from '../util';

export default function packages(state = initialState['packages'], action) {
  switch (action.type) {

    case actionTypes.CLEAR_CURRENT_PACKAGE_CHANNELS:
      return state.set('currentChannels', []);

    case actionTypes.CLEAR_PACKAGES:
      return state.set('nextRange', 0).
        set('visible', List()).
        set('totalCount', 0).
        setIn(['ui', 'visible', 'loading'], true).
        setIn(['ui', 'visible', 'exists'], false);

    case actionTypes.CLEAR_LATEST_PACKAGE:
      return state.set('latest', Package()).
        setIn(['ui', 'latest', 'loading'], true).
        setIn(['ui', 'latest', 'exists'], false);

    case actionTypes.CLEAR_LATEST_IN_CHANNEL:
      return state.setIn(['latestInChannel', action.payload.channel], undefined).
        setIn(['ui', 'latestInChannel', action.payload.channel, 'errorMessage'], undefined).
        setIn(['ui', 'latestInChannel', action.payload.channel, 'exists'], false).
        setIn(['ui', 'latestInChannel', action.payload.channel, 'loading'], true);

    case actionTypes.SET_PACKAGE_CREATING_FLAG:
      return state.setIn(['ui', 'current', 'creating'], action.payload);

    case actionTypes.POPULATE_DASHBOARD_RECENT:
      return state.setIn(['dashboard', 'recent'], List(action.payload));

    case actionTypes.CLEAR_PACKAGE_VERSIONS:
      return state.set('versions', undefined)
        .set('currentPlatforms', [])
        .setIn(['ui', 'versions', 'errorMessage'], undefined)
        .setIn(['ui', 'versions', 'loading'], true)
        .setIn(['ui', 'versions', 'exists'], false);

    case actionTypes.SET_CURRENT_PACKAGE:
      if (action.error) {
        return state.set('current', Package()).
          setIn(['ui', 'current', 'errorMessage'], action.error.message).
          setIn(['ui', 'current', 'loading'], false).
          setIn(['ui', 'current', 'exists'], false);
      } else {
        let p = Object.assign({}, action.payload);
        p.manifest = marked(p.manifest);
        // Immutable Package object has its own size property
        p.hart_size = p.size;
        delete p.size;
        return state.set('current', Package(p)).
          setIn(['ui', 'current', 'errorMessage'], undefined).
          setIn(['ui', 'current', 'exists'], true).
          setIn(['ui', 'current', 'loading'], false);
      }

    case actionTypes.SET_CURRENT_PACKAGE_TARGET:
      return state.set('currentPlatform', action.payload);

    case actionTypes.SET_CURRENT_PACKAGE_CHANNELS:
      return state.set('currentChannels', action.payload);

    case actionTypes.SET_CURRENT_PACKAGE_VERSIONS:
      if (action.error) {
        return state.set('versions', undefined).
          setIn(['ui', 'versions', 'errorMessage'],
          action.error.message).
          setIn(['ui', 'versions', 'loading'], false).
          setIn(['ui', 'versions', 'exists'], false);
      } else {
        return state.set('versions', action.payload).
          set('currentPlatforms', targetsFromPkgVersions(action.payload)).
          setIn(['ui', 'versions', 'errorMessage'], undefined).
          setIn(['ui', 'versions', 'exists'], true).
          setIn(['ui', 'versions', 'loading'], false);
      }

    case actionTypes.SET_CURRENT_PACKAGE_SETTINGS:
      if (action.error) {
        return state.set('currentSettings', undefined);
      } else {
        return state.set('currentSettings', action.payload);
      }

    case actionTypes.SET_LATEST_IN_CHANNEL:
      if (action.error) {
        return state.setIn(['latestInChannel', action.payload.channel], undefined).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'errorMessage'], action.error.message).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'exists'], false).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'loading'], false);
      } else {
        return state.setIn(['latestInChannel', action.payload.channel], Package(action.payload.pkg)).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'errorMessage'], undefined).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'exists'], true).
          setIn(['ui', 'latestInChannel', action.payload.channel, 'loading'], false);
      }

    case actionTypes.SET_LATEST_PACKAGE:
      if (action.error) {
        return state.set('latest', Package()).
          setIn(['ui', 'latest', 'errorMessage'], action.error.message).
          setIn(['ui', 'latest', 'exists'], false).
          setIn(['ui', 'latest', 'loading'], false);
      } else {
        let p = Object.assign({}, action.payload);
        p.manifest = marked(p.manifest);
         // Immutable Package object has its own size property
        p.hart_size = p.size;
        delete p.size;
        return state.set('latest', Package(p)).
          setIn(['ui', 'latest', 'errorMessage'], undefined).
          setIn(['ui', 'latest', 'exists'], true).
          setIn(['ui', 'latest', 'loading'], false);
      }

    case actionTypes.SET_PACKAGES_NEXT_RANGE:
      return state.set('nextRange', action.payload);

    case actionTypes.SET_PACKAGES_SEARCH_QUERY:
      return state.set('searchQuery', action.payload);

    case actionTypes.SET_PACKAGES_TOTAL_COUNT:
      return state.set('totalCount', action.payload);

    case actionTypes.SET_VISIBLE_PACKAGES:
      if (action.error) {
        return state.set('visible', List()).
          setIn(['ui', 'visible', 'errorMessage'], action.error.message).
          setIn(['ui', 'visible', 'exists'], false).
          setIn(['ui', 'visible', 'loading'], false);
      } else {
        return state.set('visible', state.get('visible').concat(List(action.payload))).
          setIn(['ui', 'visible', 'errorMessage'], undefined).
          setIn(['ui', 'visible', 'exists'], true).
          setIn(['ui', 'visible', 'loading'], false);
      }

    default:
      return state;
  }
}
