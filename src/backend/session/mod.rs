//!
//! Abstraction of different session apis.
//!
//! Sessions provide a way for multiple graphical systems to run in parallel by providing
//! mechanisms to switch between and handle device access and permissions for every running
//! instance.
//!
//! They are crutial to allow unpriviledged processes to use graphical or input devices.
//!
//! The following mechanisms are currently provided:
//!     - direct - legacy tty / virtual terminal kernel api
//!
use nix::fcntl::OFlag;
use std::cell::RefCell;
use std::os::unix::io::RawFd;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wayland_server::StateProxy;

/// General session interface.
///
/// Provides a way to open and close devices and change the active vt.
pub trait Session {
    /// Error type of the implementation
    type Error: AsErrno;

    /// Opens a device at the given `path` with the given flags.
    ///
    /// Returns a raw file descriptor
    fn open(&mut self, path: &Path, flags: OFlag) -> Result<RawFd, Self::Error>;
    /// Close a previously opened file descriptor
    fn close(&mut self, fd: RawFd) -> Result<(), Self::Error>;

    /// Change the currently active virtual terminal
    fn change_vt(&mut self, vt: i32) -> Result<(), Self::Error>;

    /// Check if this session is currently active
    fn is_active(&self) -> bool;
    /// Which seat this session is on
    fn seat(&self) -> String;
}

/// Interface for registering for notifications for a given session.
///
/// Part of the session api which allows to get notified, when the given session
/// gets paused or becomes active again. Any object implementing the `SessionObserver` trait
/// may be registered.
pub trait SessionNotifier {
    /// Id type of registered observers
    type Id: PartialEq + Eq;

    /// Registers a given `SessionObserver`.
    ///
    /// Returns an id of the inserted observer, can be used to remove it again.
    fn register<S: SessionObserver + 'static>(&mut self, signal: S) -> Self::Id;
    /// Removes an observer by its given id from `SessionNotifier::register`.
    fn unregister(&mut self, signal: Self::Id);

    /// Check if this session is currently active
    fn is_active(&self) -> bool;
    /// Which seat this session is on
    fn seat(&self) -> &str;
}

/// Trait describing the ability to be notified when the session pauses or becomes active again.
///
/// It might be impossible to interact with devices while the session is disabled.
/// This interface provides callbacks for when that happens.
pub trait SessionObserver {
    /// Session is about to be paused.
    ///
    /// In case the implementor is a `StateToken` the state of the `EventLoop`
    /// is provided via a `StateProxy`.
    fn pause<'a>(&mut self, state: &mut StateProxy<'a>);
    /// Session got active again
    ///
    /// In case the implementor is a `StateToken` the state of the `EventLoop`
    /// is provided via a `StateProxy`.
    fn activate<'a>(&mut self, state: &mut StateProxy<'a>);
}

impl Session for () {
    type Error = ();

    fn open(&mut self, _path: &Path, _flags: OFlag) -> Result<RawFd, Self::Error> {
        Err(())
    }
    fn close(&mut self, _fd: RawFd) -> Result<(), Self::Error> {
        Err(())
    }

    fn change_vt(&mut self, _vt: i32) -> Result<(), Self::Error> {
        Err(())
    }

    fn is_active(&self) -> bool {
        false
    }
    fn seat(&self) -> String {
        String::from("seat0")
    }
}

impl<S: Session> Session for Rc<RefCell<S>> {
    type Error = S::Error;

    fn open(&mut self, path: &Path, flags: OFlag) -> Result<RawFd, Self::Error> {
        self.borrow_mut().open(path, flags)
    }

    fn close(&mut self, fd: RawFd) -> Result<(), Self::Error> {
        self.borrow_mut().close(fd)
    }

    fn change_vt(&mut self, vt: i32) -> Result<(), Self::Error> {
        self.borrow_mut().change_vt(vt)
    }

    fn is_active(&self) -> bool {
        self.borrow().is_active()
    }

    fn seat(&self) -> String {
        self.borrow().seat()
    }
}

impl<S: Session> Session for Arc<Mutex<S>> {
    type Error = S::Error;

    fn open(&mut self, path: &Path, flags: OFlag) -> Result<RawFd, Self::Error> {
        self.lock().unwrap().open(path, flags)
    }

    fn close(&mut self, fd: RawFd) -> Result<(), Self::Error> {
        self.lock().unwrap().close(fd)
    }

    fn change_vt(&mut self, vt: i32) -> Result<(), Self::Error> {
        self.lock().unwrap().change_vt(vt)
    }

    fn is_active(&self) -> bool {
        self.lock().unwrap().is_active()
    }

    fn seat(&self) -> String {
        self.lock().unwrap().seat()
    }
}

/// Allows errors to be described by an error number
pub trait AsErrno: ::std::fmt::Debug {
    /// Returns the error number representing this error if any
    fn as_errno(&self) -> Option<i32>;
}

impl AsErrno for () {
    fn as_errno(&self) -> Option<i32> {
        None
    }
}

pub mod direct;
