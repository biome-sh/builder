enum OAuthProviderType {
  ActiveDirectory = 'active-directory',
  AzureAD = 'azure-ad',
  ChefAutomate = 'chef-automate',
  GitHub = 'github',
  GitLab = 'gitlab',
  Bitbucket = 'bitbucket',
  Okta = 'okta',
}

export abstract class OAuthProvider {
  static providers = Object.keys(OAuthProviderType).map(key => `'${OAuthProviderType[key]}'`).join(', ');

  public name;

  constructor(
    public type: OAuthProviderType,
    public clientID: string,
    public authorizeUrl: string,
    public redirectUrl: string,
    public signupUrl: string,
    public useState: boolean,
    public params: object) {

    if (!this.type) {
      console.error(`Please configure Builder with an OAuth provider. Supported providers are ${OAuthProvider.providers}.`);
    }

    if (!this.clientID) {
      console.error(`Please configure Builder with your OAuth application's client ID.`);
    }

    if (!this.authorizeUrl) {
      console.error(`Please configure Builder with an OAuth authorization URL. (e.g., 'https://github.com/login/oauth/authorize')`);
    }

    if (!this.redirectUrl) {
      console.error(`Please configure Builder with an OAuth redirect URL. (e.g., 'https://yourdomain.com/oauth/redirect')`);
    }

    if (!this.signupUrl) {
      console.warn(`Consider configuring Builder with an OAuth signup URL for your users. (e.g., 'https://github.com/join').`);
    }
  }

  static fromConfig(type: string, clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string): OAuthProvider {
    switch (type) {
      case OAuthProviderType.ActiveDirectory:
        return new ActiveDirectoryProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
      case OAuthProviderType.AzureAD:
        return new AzureADProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
        case OAuthProviderType.ChefAutomate:
        return new ChefAutomateProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
      case OAuthProviderType.GitHub:
        return new GitHubProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
      case OAuthProviderType.GitLab:
        return new GitLabProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
      case OAuthProviderType.Bitbucket:
        return new BitbucketProvider(clientID, authorizeUrl, redirectUrl, signupUrl);
      case OAuthProviderType.Okta:
        return new OktaProvider(clientID, authorizeUrl, redirectUrl, signupUrl, state);
      case undefined:
      case '':
        console.error(`Please configure Builder with an OAuth provider. Supported providers are ${OAuthProvider.providers}.`);
        break;
      default:
        console.error(`Unsupported OAuth provider '${type}'. Supported providers are ${OAuthProvider.providers}.`);
    }
  }
}

class ActiveDirectoryProvider extends OAuthProvider {
  name: string = 'Active Directory';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.ActiveDirectory,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state,
        scope: 'openid',
        nonce: 0 // Ok since we're not using id_token
      }
    );
  }
}

class AzureADProvider extends OAuthProvider {
  name: string = 'Azure AD';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.AzureAD,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state,
        scope: 'openid',
        nonce: 0 // Ok since we're not using id_token
      }
    );
  }
}

class ChefAutomateProvider extends OAuthProvider {
  name: string = 'Cinc Automate';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.ChefAutomate,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state,
        scope: 'openid profile email',
        nonce: 0
      }
    );
  }
}

class GitHubProvider extends OAuthProvider {
  name: string = 'GitHub';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.GitHub,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state
      }
    );
  }
}

class GitLabProvider extends OAuthProvider {
  name: string = 'GitLab';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.GitLab,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state,
        scope: 'openid'
      }
    );
  }
}

class BitbucketProvider extends OAuthProvider {
  name: string = 'Bitbucket';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string) {
    super(
      OAuthProviderType.Bitbucket,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      false,
      {
        client_id: clientID,
        response_type: 'code'
      }
    );
  }
}

class OktaProvider extends OAuthProvider {
  name: string = 'Okta';

  constructor(clientID: string, authorizeUrl: string, redirectUrl: string, signupUrl: string, state: string) {
    super(
      OAuthProviderType.Okta,
      clientID,
      authorizeUrl,
      redirectUrl,
      signupUrl,
      true,
      {
        client_id: clientID,
        redirect_uri: redirectUrl,
        response_type: 'code',
        state: state,
        scope: 'openid profile email',
        nonce: 0
      }
    );
  }
}
