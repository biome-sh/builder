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

import { Injectable } from '@angular/core';
import { applyMiddleware, compose, createStore, Store } from 'redux';
import rootReducer from './reducers/index';
import thunk from 'redux-thunk';
import reduxReset from 'redux-reset';
import { Observable, BehaviorSubject } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import { get, isEqual } from 'lodash';

const composeEnhancers = window['__REDUX_DEVTOOLS_EXTENSION_COMPOSE__'] || compose;

const finalCreateStore = composeEnhancers(
  // The thunk middleware allows an action to return a function that takes a
  // dispatch argument instead of returning an object directly. This allows
  // actions to make async calls.
  applyMiddleware(thunk),

  // Allows resetting of the store
  reduxReset()
)(createStore);

const appStore = finalCreateStore(rootReducer);

@Injectable()
export class AppStore {
  private store: Store = appStore;

  private store$: Observable<any>;

  private storeSource$: BehaviorSubject<any>;

  constructor() {
    this.storeSource$ = new BehaviorSubject(this.store.getState());
    this.store$ = this.storeSource$.asObservable().pipe(distinctUntilChanged(isEqual));
    this.store.subscribe(() => this.storeSource$.next(this.store.getState()));
  }

  getState(): any {
    return this.store.getState();
  }

  dispatch(action) {
    this.store.dispatch(action);
  }

  observe(path?: string | string[]): Observable<any> {
    return this.store$.pipe(
      map(state => path ? get(state, path) : state),
      distinctUntilChanged(isEqual)
    );
  }

  subscribe(listener: Function) {
    return this.store.subscribe(() => listener(this.getState()));
  }
}
