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

import * as cookies from 'js-cookie';
import * as actions from './actions/index';
import * as depotApi from './client/depot-api';
import { Browser } from './browser';

describe('actions', () => {

  xdescribe('populateBuildLog', () => {
    describe('when data is undefined', () => {
      it('has an undefined payload', () => {

      });
    });

    describe('when data is a string', () => {
      it('has a string payload', () => {

      });
    });
  });

  describe('filterPackagesBy', () => {

    describe('given a query parameter', () => {

      it('encodes the parameter before sending it', () => {
        spyOn(depotApi, 'get').and.returnValue(new Promise(() => { }));
        actions.filterPackagesBy({}, 'core/awesome', false)(() => { });
        expect(depotApi.get).toHaveBeenCalledWith({ query: 'core%2Fawesome' }, 0);
      });
    });
  });

  describe('Browser', () => {

    describe('setCookie', () => {

      it('applies the proper domain', () => {
        spyOn(cookies, 'set');

        spyOnProperty(Browser, 'currentHostname', 'get').and.returnValues(
          'localhost',
          'builder.biome.sh',
          'builder.acceptance.biome.foo',
          '1.2.3.4'
        );

        Browser.setCookie('oauthToken', 'some-token');
        Browser.setCookie('oauthToken', 'some-token');
        Browser.setCookie('oauthToken', 'some-token');
        Browser.setCookie('oauthToken', 'some-token');

        expect(cookies.set.calls.allArgs()).toEqual(
          [
            ['oauthToken', 'some-token', { domain: 'localhost', secure: false }],
            ['oauthToken', 'some-token', { domain: 'builder.biome.sh', secure: false }],
            ['oauthToken', 'some-token', { domain: 'builder.acceptance.biome.foo', secure: false }],
            ['oauthToken', 'some-token', { domain: '1.2.3.4', secure: false }]
          ]
        );
      });
    });
  });
});
