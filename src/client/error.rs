use crate::private_key::PrivateKeyError;
use prost::DecodeError;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use tonic::transport::Error as TonicError;
use tonic::Status;

#[derive(Debug)]
pub enum CosmosGrpcError {
    NoToken,
    BadResponse(String),
    BadStruct(String),
    SigningError { error: PrivateKeyError },
    ConnectionError { error: TonicError },
    RequestError { error: Status },
    DecodeError { error: DecodeError },
    BadInput(String),
    ChainNotRunning,
    NodeNotSynced,
}

impl Display for CosmosGrpcError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            CosmosGrpcError::NoToken => {
                write!(f, "Account has no tokens! No details!")
            }
            CosmosGrpcError::BadResponse(val) => write!(f, "CosmosGrpc bad response {}", val),
            CosmosGrpcError::BadStruct(val) => {
                write!(f, "CosmosGrpc unexpected json returned {}", val)
            }
            CosmosGrpcError::BadInput(val) => write!(f, "CosmosGrpc bad input {}", val),
            CosmosGrpcError::DecodeError { error: val } => {
                write!(f, "CosmosGrpc bad any unpacking {}", val)
            }
            CosmosGrpcError::ConnectionError { error } => {
                write!(f, "CosmosGrpc Connection error {} {:?}", error, error)
            }
            CosmosGrpcError::RequestError { error } => {
                write!(f, "CosmosGrpc Request error {} {:?}", error, error)
            }
            CosmosGrpcError::ChainNotRunning => {
                write!(f, "CosmosGrpc this node is waiting on a blockchain start")
            }
            CosmosGrpcError::NodeNotSynced => {
                write!(f, "CosmosGrpc this node is syncing")
            }
            CosmosGrpcError::SigningError { error } => {
                write!(f, "CosmosGrpc could not sign using private key {:?}", error)
            }
        }
    }
}

impl Error for CosmosGrpcError {}

impl From<TonicError> for CosmosGrpcError {
    fn from(error: TonicError) -> Self {
        CosmosGrpcError::ConnectionError { error }
    }
}

impl From<Status> for CosmosGrpcError {
    fn from(error: Status) -> Self {
        CosmosGrpcError::RequestError { error }
    }
}

impl From<DecodeError> for CosmosGrpcError {
    fn from(error: DecodeError) -> Self {
        CosmosGrpcError::DecodeError { error }
    }
}

impl From<PrivateKeyError> for CosmosGrpcError {
    fn from(error: PrivateKeyError) -> Self {
        CosmosGrpcError::SigningError { error }
    }
}
