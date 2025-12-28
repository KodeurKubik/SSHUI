use russh::{MethodKind, MethodSet, server::Auth};

/// Result of an authentication attempt.
pub enum AuthDecision {
    /// Authentication succeeded
    Accept,
    /// Authentication failed
    Reject,
}

/// Handler for SSH authentication methods.
#[async_trait::async_trait]
pub trait AuthHandler: Send + Sync + 'static {
    /// Handle authentication with no password (none method)
    async fn auth_none(&self, _user: &str) -> AuthDecision {
        AuthDecision::Reject
    }
    /// Handle password authentication
    async fn auth_password(&self, _user: &str, _password: &str) -> AuthDecision {
        AuthDecision::Reject
    }
}

/// Authentication handler that accepts all authentication attempts
pub struct NoAuth;

#[async_trait::async_trait]
impl AuthHandler for NoAuth {
    async fn auth_none(&self, _user: &str) -> AuthDecision {
        AuthDecision::Accept
    }
    async fn auth_password(&self, _: &str, _: &str) -> AuthDecision {
        AuthDecision::Accept
    }
}

#[derive(PartialEq)]
pub(crate) enum AuthMethod {
    None,
    Password,
}

pub(crate) fn auth_to_decision(auth: AuthDecision, method: AuthMethod) -> Auth {
    match auth {
        AuthDecision::Accept => Auth::Accept,
        AuthDecision::Reject => {
            if method == AuthMethod::None {
                let mut method = MethodSet::empty();
                method.push(MethodKind::Password);

                return Auth::Reject {
                    proceed_with_methods: Some(method),
                    partial_success: false,
                };
            } else {
                Auth::Reject {
                    proceed_with_methods: None,
                    partial_success: false,
                }
            }
        }
    }
}
