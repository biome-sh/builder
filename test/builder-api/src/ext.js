const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

// These magic values correspond to the testpp repo in the biome-sh org
const installationId = 56940;
const repoId = 114932712;

describe('External API', function () {
  describe('Validate credentials in an external registry', function () {
    it('requires authentication', function (done) {
      request.post('/ext/integrations/docker/credentials/validate')
        .accept('application/json')
        .send({})
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    // This one requires correct docker creds to test and I'm not sure how to
    // include those in these tests w/o leaking them to everyone.
    it('succeeds');
  });
});
