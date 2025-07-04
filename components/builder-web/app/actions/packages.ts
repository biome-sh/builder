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

import { groupBy } from 'lodash';
import * as depotApi from '../client/depot-api';
import { BuilderApiClient } from '../client/builder-api';
import { addNotification, SUCCESS, DANGER } from './notifications';
import { latestBase } from  '../util';

export const SET_PACKAGE_CREATING_FLAG = 'SET_PACKAGE_CREATING_FLAG';
export const CLEAR_CURRENT_PACKAGE_CHANNELS = 'CLEAR_CURRENT_PACKAGE_CHANNELS';
export const CLEAR_PACKAGES = 'CLEAR_PACKAGES';
export const CLEAR_LATEST_IN_CHANNEL = 'CLEAR_LATEST_IN_CHANNEL';
export const CLEAR_LATEST_PACKAGE = 'CLEAR_LATEST_PACKAGE';
export const CLEAR_PACKAGE_VERSIONS = 'CLEAR_PACKAGE_VERSIONS';
export const POPULATE_DASHBOARD_RECENT = 'POPULATE_DASHBOARD_RECENT';
export const SET_CURRENT_PACKAGE = 'SET_CURRENT_PACKAGE';
export const SET_CURRENT_PACKAGE_TARGET = 'SET_CURRENT_PACKAGE_TARGET';
export const SET_CURRENT_PACKAGE_TARGETS = 'SET_CURRENT_PACKAGE_TARGETS';
export const SET_LATEST_IN_CHANNEL = 'SET_LATEST_IN_CHANNEL';
export const SET_LATEST_PACKAGE = 'SET_LATEST_PACKAGE';
export const SET_CURRENT_PACKAGE_CHANNELS = 'SET_CURRENT_PACKAGE_CHANNELS';
export const SET_CURRENT_PACKAGE_SETTINGS = 'SET_CURRENT_PACKAGE_SETTINGS';
export const SET_CURRENT_PACKAGE_VERSIONS = 'SET_CURRENT_PACKAGE_VERSIONS';
export const SET_PACKAGES_NEXT_RANGE = 'SET_PACKAGES_NEXT_RANGE';
export const SET_PACKAGES_SEARCH_QUERY = 'SET_PACKAGES_SEARCH_QUERY';
export const SET_PACKAGES_TOTAL_COUNT = 'SET_PACKAGES_TOTAL_COUNT';
export const SET_VISIBLE_PACKAGES = 'SET_VISIBLE_PACKAGES';
export const SET_VISIBLE_PACKAGE_CHANNELS = 'SET_VISIBLE_PACKAGE_CHANNELS';

function clearPackages() {
  return {
    type: CLEAR_PACKAGES,
  };
}

function clearCurrentPackageChannels() {
  return {
    type: CLEAR_CURRENT_PACKAGE_CHANNELS
  };
}

function clearLatestInChannel(channel: string) {
  return {
    type: CLEAR_LATEST_IN_CHANNEL,
    payload: { channel }
  };
}

function clearLatestPackage() {
  return {
    type: CLEAR_LATEST_PACKAGE
  };
}

export function fetchDashboardRecent(origin: string) {
  return dispatch => {
    return depotApi.get({ origin: origin })
      .then(data => dispatch(populateDashboardRecent(data)))
      .catch(error => console.error(error));
  };
}

export function clearPackageVersions() {
  return {
    type: CLEAR_PACKAGE_VERSIONS
  };
}

function setPackageCreatingFlag(payload) {
  return {
    type: SET_PACKAGE_CREATING_FLAG,
    payload,
  };
}

export function createEmptyPackage(body: object, token: string, callback: Function = (newPackage) => { }) {
  return dispatch => {
    dispatch(setPackageCreatingFlag(true));

    new BuilderApiClient(token).createEmptyPackage(body).then((newPackage: any) => {
      dispatch(setPackageCreatingFlag(false));

      dispatch(addNotification({
        title: 'Package created',
        body: `${newPackage.name} successfully created`,
        type: SUCCESS,
      }));

      callback(newPackage);

    }).catch(error => {
      dispatch(setPackageCreatingFlag(false));
      dispatch(addNotification({
        title: 'Failed to create package',
        body: error.message,
        type: DANGER,
      }));
    });
  };
}

export function demotePackage(origin: string, name: string, version: string, release: string, target: string, channel: string, token: string) {
  return dispatch => {
    depotApi.demotePackage(origin, name, version, release, target, channel, token)
      .then(response => {
        dispatch(addNotification({
          title: 'Package demoted',
          body: `${origin}/${name}/${version}/${release} (${target}) has been removed from the ${channel} channel.`,
          type: SUCCESS
        }));
        dispatch(fetchLatestInChannel(origin, name, 'stable', target));
        dispatch(fetchLatestInChannel(origin, name, latestBase, target));
        dispatch(fetchPackageChannels(origin, name, version, release, target));
        dispatch(fetchPackageVersions(origin, name));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to demote package',
          body: `There was an error removing ${origin}/${name}/${version}/${release}
            from the ${channel} channel. The message was ${error.message}.`,
          type: DANGER
        }));
      });
  };
}

export function fetchPackage(pkg) {
  return dispatch => {
    depotApi.get(pkg.ident).then(response => {
      const pkg = response['results'];
      dispatch(setCurrentPackage(pkg));
      dispatch(fetchPackageChannels(pkg.ident.origin, pkg.ident.name, pkg.ident.version, pkg.ident.release, pkg.target));
    }).catch(error => {
      dispatch(setCurrentPackage(undefined, error));
    });
  };
}

export function fetchPackageChannels(origin: string, name: string, version: string, release: string, target: string = '') {
  return dispatch => {
    dispatch(clearCurrentPackageChannels());

    depotApi.getPackageChannels(origin, name, version, release, target)
      .then(response => {
        dispatch(setCurrentPackageChannels(response));
      })
      .catch(error => console.error(error));
  };
}

export function fetchLatestPackage(origin: string, name: string, target: string) {
  return dispatch => {
    dispatch(clearLatestPackage());

    depotApi.getLatest(origin, name, target).then(response => {
      dispatch(setLatestPackage(response));

      const ident = response['ident'];
      dispatch(fetchPackageChannels(ident.origin, ident.name, ident.version, ident.release, target));
    }).catch(error => {
      dispatch(setLatestPackage(undefined, error));
    });
  };
}

export function fetchLatestInChannel(origin: string, name: string, channel: string, target: string) {
  return dispatch => {
    dispatch(clearLatestInChannel(channel));

    depotApi.getLatestInChannel(origin, name, channel, target)
      .then(response => {
        dispatch(setLatestInChannel(channel, response));
      })
      .catch(error => {
        dispatch(setLatestInChannel(channel, undefined, error));
      });
  };
}

export function fetchPackageSettings(origin: string, name: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).getPackageSettings(origin, name)
      .then(settings => dispatch(setCurrentPackageSettings(settings)))
      .catch(error => dispatch(setCurrentPackageSettings({}, error)));
  };
}

export function fetchPackageVersions(origin: string, name: string) {
  return dispatch => {
    dispatch(clearPackages());
    dispatch(clearPackageVersions());
    depotApi.getPackageVersions(origin, name).then(response => {
      dispatch(setCurrentPackageVersions(response));
    }).catch(error => {
      dispatch(setCurrentPackageVersions(undefined, error));
    });
  };
}

export function getUniquePackages(
  origin: string,
  nextRange: number = 0,
  token: string = ''
) {
  return dispatch => {
    if (nextRange === 0) {
      dispatch(clearPackages());
    }

    depotApi.getUnique(origin, nextRange, token).then(response => {
      dispatch(setVisiblePackages(response['results']));
      dispatch(setPackagesTotalCount(response['totalCount']));
      dispatch(setPackagesNextRange(response['nextRange']));
    }).catch(error => {
      dispatch(setVisiblePackages(undefined, error));
    });
  };
}

export function filterPackagesBy(
  params,
  query: string,
  distinct: boolean,
  nextRange: number = 0
) {
  return dispatch => {
    // We send -1 for fetching all version pacakges
    if (nextRange <= 0) {
      dispatch(clearPackages());
    }

    if (query) {
      params.query = encodeURIComponent(query);
    }

    if (distinct) {
      params.distinct = true;
    }

    depotApi.get(params, nextRange).then(response => {
      dispatch(setVisiblePackages(response['results']));
      dispatch(setPackagesTotalCount(response['totalCount']));
      dispatch(setPackagesNextRange(response['nextRange']));
    }).catch(error => {
      dispatch(setVisiblePackages(undefined, error));
    });
  };
}

export function populateDashboardRecent(data) {
  let grouped = groupBy(data.results.package_list.reverse(), 'name');
  let mapped = [];

  for (let k in grouped) {
    mapped.push(grouped[k][0]);
  }

  return {
    type: POPULATE_DASHBOARD_RECENT,
    payload: mapped
  };
}

export function promotePackage(origin: string, name: string, version: string, release: string, target: string, channel: string, token: string) {
  return dispatch => {
    depotApi.promotePackage(origin, name, version, release, target, channel, token)
      .then(response => {
        dispatch(addNotification({
          title: 'Package promoted',
          body: `${origin}/${name}/${version}/${release} (${target}) has been promoted to the ${channel} channel.`,
          type: SUCCESS
        }));
        dispatch(fetchLatestInChannel(origin, name, 'stable', target));
        dispatch(fetchLatestInChannel(origin, name, latestBase, target));
        dispatch(fetchPackageChannels(origin, name, version, release, target));
        dispatch(fetchPackageVersions(origin, name));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to promote package',
          body: `There was an error promoting ${origin}/${name}/${version}/${release}
            to the ${channel} channel. The message was ${error.message}.`,
          type: DANGER
        }));
      });
  };
}

export function setCurrentPackage(pkg, error = undefined) {
  return {
    type: SET_CURRENT_PACKAGE,
    payload: pkg,
    error: error,
  };
}

export function setCurrentPackageTarget(payload) {
  return {
    type: SET_CURRENT_PACKAGE_TARGET,
    payload
  };
}


export function setCurrentPackageTargets(payload) {
  return {
    type: SET_CURRENT_PACKAGE_TARGETS,
    payload
  };
}

export function setLatestPackage(pkg, error = undefined) {
  return {
    type: SET_LATEST_PACKAGE,
    payload: pkg,
    error: error,
  };
}

export function setLatestInChannel(channel, pkg, error = undefined) {
  return {
    type: SET_LATEST_IN_CHANNEL,
    payload: { channel, pkg },
    error: error,
  };
}

export function setCurrentPackageChannels(channels) {
  return {
    type: SET_CURRENT_PACKAGE_CHANNELS,
    payload: channels
  };
}

export function setCurrentPackageSettings(settings, error = undefined) {
  return {
    type: SET_CURRENT_PACKAGE_SETTINGS,
    payload: settings,
    error: error,
  };
}

export function setCurrentPackageVisibility(origin: string, name: string, setting: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).setPackageVisibility(origin, name, setting)
      .then(settings => {
        dispatch(setCurrentPackageSettings(settings));
        dispatch(addNotification({
          title: 'Privacy settings saved',
          type: SUCCESS
        }));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to save privacy settings',
          body: error.message,
          type: DANGER
        }));
      });
  };
}

export function setCurrentPackageVersions(versions, error = undefined) {
  return {
    type: SET_CURRENT_PACKAGE_VERSIONS,
    payload: versions,
    error: error,
  };
}

function setPackagesNextRange(payload: number) {
  return {
    type: SET_PACKAGES_NEXT_RANGE,
    payload,
  };
}

export function setPackagesSearchQuery(payload: string) {
  return {
    type: SET_PACKAGES_SEARCH_QUERY,
    payload,
  };
}

function setPackagesTotalCount(payload: number) {
  return {
    type: SET_PACKAGES_TOTAL_COUNT,
    payload,
  };
}

export function setVisiblePackages(params, error = undefined) {
  return {
    type: SET_VISIBLE_PACKAGES,
    payload: params,
    error: error,
  };
}

export function setPackageReleaseVisibility(origin: string, name: string, version: string, release: string, setting: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).setPackageReleaseVisibility(origin, name, version, release, setting)
      .then(response => {
        const ident = { origin, name, version, release };
        dispatch(fetchPackage({ ident }));
        dispatch(addNotification({
          title: 'Privacy settings saved',
          type: SUCCESS
        }));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to save privacy settings',
          body: error.message,
          type: DANGER
        }));
      });
  };
}
