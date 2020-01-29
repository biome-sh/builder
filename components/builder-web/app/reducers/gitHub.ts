// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

import { fromJS, List } from 'immutable';
import initialState from '../initial-state';
import * as actionTypes from '../actions/index';

export default function gitHub(state = initialState['gitHub'], action) {
  switch (action.type) {

    case actionTypes.CLEAR_GITHUB_INSTALLATIONS:
      return state.set('installations', List()).
        setIn(['ui', 'installations', 'loading'], true);

    case actionTypes.CLEAR_GITHUB_REPOSITORIES:
        return state.set('repositories', List()).
          setIn(['ui', 'repositories', 'loading'], true);

    case actionTypes.POPULATE_GITHUB_INSTALLATIONS:
      return state.set('installations', fromJS(action.payload)).
        setIn(['ui', 'installations', 'loading'], false);

    case actionTypes.POPULATE_GITHUB_REPOSITORIES:
      return state.set('repositories', fromJS(action.payload)).
        setIn(['ui', 'repositories', 'loading'], false);

    default:
      return state;
  }
}
