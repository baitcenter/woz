use std::collections::HashMap;
use rusoto_core::{RusotoFuture, RusotoError};
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::CognitoIdentityProvider;
use rusoto_cognito_idp::*;

use crate::cache::FileCache;
use crate::config::*;


pub fn signup(client: &CognitoIdentityProviderClient, email: String,
              username: String, password: String)
              -> RusotoFuture<SignUpResponse, SignUpError> {
    let mut request = SignUpRequest::default();
    request.username = username;
    request.password = password;
    request.client_id = String::from(CLIENT_ID);
    let email = AttributeType {
        name: String::from("email"),
        value: Some(email)
    };
    request.user_attributes = Some(vec![email]);
    client.sign_up(request)
}

pub fn login(client: &CognitoIdentityProviderClient, username: String, password: String) -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut request = InitiateAuthRequest::default();
    request.auth_flow = String::from("USER_PASSWORD_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("USERNAME"), username);
    auth_params.insert(String::from("PASSWORD"), password);
    request.client_id = String::from(CLIENT_ID);
    request.auth_parameters = Some(auth_params);
    client.initiate_auth(request)
}

pub fn refresh_auth(client: &CognitoIdentityProviderClient, refresh_token: &str)
                    -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("REFRESH_TOKEN"), refresh_token.to_string());
    let req = InitiateAuthRequest {
        client_id: CLIENT_ID.to_string(),
        auth_flow: String::from("REFRESH_TOKEN_AUTH"),
        auth_parameters: Some(auth_params),
        ..Default::default()
    };
    client.initiate_auth(req)
}

pub fn identity_id(client: &CognitoIdentityClient, id_token: &str)
                   -> RusotoFuture<GetIdResponse, GetIdError> {
    let mut logins = HashMap::new();
     logins.insert(USER_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetIdInput::default();
    req.identity_pool_id = IDENTITY_POOL_ID.to_string();
    req.logins = Some(logins);
    client.get_id(req)
}

/// After a user has been signed up via `signup`, set up their account
/// by generating and storing an identity and refresh token. Result
/// will fail if the user has not confirmed their email address.
pub fn setup(id_provider_client: &CognitoIdentityProviderClient,
             id_client: &CognitoIdentityClient,
             cache: &FileCache,
             username: String, password: String) -> Result<(), InitiateAuthError> {
    login(&id_provider_client, username, password)
        .sync()
        .map(|resp| {
            let auth_result = resp.authentication_result
                .expect("No auth result");

            // Store the refresh token
            let refresh_token = auth_result.refresh_token
                .expect("No access token found");

            cache.set_encrypted("refresh_token", refresh_token.as_bytes().to_vec())
                .expect("Failed to set refresh token in cache");

            // Store the identity ID
            let id_token = auth_result.id_token
                .expect("No ID token found");

            let identity_id = identity_id(&id_client, &id_token)
                .sync()
                .expect("Getting identity ID didn't work")
                .identity_id.expect("No identity ID");

            cache.set("identity", identity_id.as_bytes().to_vec())
                .expect("Failed to set identity ID in cache");
        })
        .map_err(|err| match err {
            RusotoError::Service(e) => e,
            _ => panic!("Unknown error")
        })
}

type AWSCredentialsResponse = RusotoFuture<GetCredentialsForIdentityResponse,
                                           GetCredentialsForIdentityError>;
pub fn aws_credentials(client: &CognitoIdentityClient, identity_id: &str, id_token: &str)
                       ->  AWSCredentialsResponse {
    let mut logins = HashMap::new();
    logins.insert(USER_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetCredentialsForIdentityInput::default();
    req.identity_id = identity_id.to_owned();
    req.logins = Some(logins);
    client.get_credentials_for_identity(req)
}
