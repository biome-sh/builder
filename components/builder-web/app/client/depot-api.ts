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

import 'whatwg-fetch';
import { packageString } from '../util';
import { AppStore } from '../app.store';
import { addNotification, signOut } from '../actions/index';
import { WARNING } from '../actions/notifications';

const urlPrefix = 'v1';

function opts() {
  const store = new AppStore();
  let token = store.getState().session.token;
  let o: any = {};

  if (token) {
    o.headers = {
      Authorization: `Bearer ${token}`
    };
  }

  return o;
}

function handleError(error, reject) {
  const store = new AppStore();
  const state = store.getState();
  store.dispatch(signOut(true, state.router.route.url));
  reject(error);

  if (state.session.token) {
    setTimeout(() => {
      store.dispatch(addNotification({
        title: 'Session Expired',
        body: 'Please sign in again.',
        type: WARNING
      }));
    }, 1000);
  }
}

function handleUnauthorized(response, reject) {
  if (response.status === 401) {
    throw new Error('Unauthorized');
  }

  return response;
}

export function demotePackage(origin: string, name: string, version: string, release: string, target: string, channel: string, token: string) {
  const url = `${urlPrefix}/depot/channels/${origin}/${channel}/pkgs/${name}/${version}/${release}/demote?target=${target}`;

  return new Promise((resolve, reject) => {
    fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
      method: 'PUT',
    })
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.ok) {
          resolve(true);
        } else {
          reject(new Error(response.statusText));
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getUnique(origin: string, nextRange: number = 0, token: string = '') {
  const url = `${urlPrefix}/depot/${origin}/pkgs?range=${nextRange}`;

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(resultsObj => {
            let results;

            const endRange = parseInt(resultsObj.range_end, 10);
            const totalCount = parseInt(resultsObj.total_count, 10);
            const nextRange = totalCount > (endRange + 1) ? endRange + 1 : 0;

            if (resultsObj['data']) {
              results = resultsObj['data'];
            } else {
              results = resultsObj;
            }

            resolve({ results, totalCount, nextRange });
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getLatest(origin: string, pkg: string, target: string) {
  const url = `${urlPrefix}/depot/pkgs/${origin}/${pkg}/latest?target=${target}`;

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(results => {
            resolve(results);
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getLatestInChannel(origin: string, name: string, channel: string, target: string) {
  const tgt = target ? `?target=${target}` : ``;
  const url = `${urlPrefix}/depot/channels/${origin}/${channel}/pkgs/${name}/latest${tgt}`;

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(results => {
            resolve(results);
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function get(params, nextRange: number = 0) {
  let url = `${urlPrefix}/depot/pkgs/` +
    (params['query'] ? `search/${params['query']}`
      : packageString(params)) +
    `?range=${nextRange}`;

  if (params['distinct']) {
    url += '&distinct=true';
  }

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        // Fail the promise if an error happens.
        //
        // If we're hitting the fake api, the 4xx response will show up
        // here, but if we're hitting the real Builder, it will show up in the
        // catch below.
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(resultsObj => {
            let results;

            const endRange = parseInt(resultsObj.range_end, 10);
            const totalCount = parseInt(resultsObj.total_count, 10);
            const nextRange = totalCount > (endRange + 1) ? endRange + 1 : 0;

            if (resultsObj['data']) {
              results = resultsObj['data'];
            } else {
              results = resultsObj;
            }

            resolve({ results, totalCount, nextRange });
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getPackageChannels(origin: string, name: string, version: string, release: string, target: string = '') {
  let url = `${urlPrefix}/depot/pkgs/${origin}/${name}/${version}/${release}/channels`;
  if (target) {
    url = `${url}?target=${target}`;
  }

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(results => {
            resolve(results);
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getPackageVersions(origin: string, pkg: string) {
  const url = `${urlPrefix}/depot/pkgs/${origin}/${pkg}/versions`;

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        }
        else {
          response.json().then(results => {
            resolve(results);
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function promotePackage(origin: string, name: string, version: string, release: string, target: string, channel: string, token: string) {
  const url = `${urlPrefix}/depot/channels/${origin}/${channel}/pkgs/${name}/${version}/${release}/promote?target=${target}`;

  return new Promise((resolve, reject) => {
    fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
      method: 'PUT',
    })
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.ok) {
          resolve(true);
        } else {
          reject(new Error(response.statusText));
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function submitJob(origin: string, pkg: string, target: string, token: string) {
  const url = `${urlPrefix}/depot/pkgs/schedule/${origin}/${pkg}?target=${target}`;

  return new Promise((resolve, reject) => {
    fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
      method: 'POST',
    })
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.ok) {
          resolve(true);
        } else {
          reject(new Error(response.statusText));
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getEvents(nextRange: number = 0, fromDate: string, toDate: string, query: string = '') {
  let url = `${urlPrefix}/depot/events` + `?range=${nextRange}&channel=stable&from_date=${fromDate}&to_date=${toDate}&query=${query}`;

  return new Promise((resolve, reject) => {
    fetch(url, opts())
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        } else {
          response.json().then(resultsObj => {
            let results;

            const endRange = parseInt(resultsObj.range_end, 10);
            const totalCount = parseInt(resultsObj.total_count, 10);
            const nextRange = totalCount > (endRange + 1) ? endRange + 1 : 0;

            if (resultsObj['data']) {
              results = resultsObj['data'];
            } else {
              results = resultsObj;
            }

            resolve({ results, totalCount, nextRange });
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}

export function getSaasEvents(nextRange: number = 0, fromDate: string, toDate: string, query: string = '') {
  let url = `${urlPrefix}/depot/events/saas?range=${nextRange}&channel=stable&from_date=${fromDate}&to_date=${toDate}&query=${query}`;

  return new Promise((resolve, reject) => {
    fetch(url)
      .then(response => handleUnauthorized(response, reject))
      .then(response => {
        if (response.status >= 400) {
          reject(new Error(response.statusText));
        } else {
          response.json().then(resultsObj => {
            let results;

            const endRange = parseInt(resultsObj.range_end, 10);
            const totalCount = parseInt(resultsObj.total_count, 10);
            const nextRange = totalCount > (endRange + 1) ? endRange + 1 : 0;

            if (resultsObj['data']) {
              results = resultsObj['data'];
            } else {
              results = resultsObj;
            }

            resolve({ results, totalCount, nextRange });
          });
        }
      })
      .catch(error => handleError(error, reject));
  });
}
