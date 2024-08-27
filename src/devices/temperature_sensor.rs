// phidget-rs/src/temperature_sensor.rs
//
// Copyright (c) 2023, Frank Pagliughi
//
// This file is part of the 'phidget-rs' library.
//
// Licensed under the MIT license:
//   <LICENSE or http://opensource.org/licenses/MIT>
// This file may not be copied, modified, or distributed except according
// to those terms.
//

use crate::{AttachCallback, DetachCallback, GenericPhidget, Phidget, Result, ReturnCode};
use phidget_sys::{
    self as ffi, PhidgetHandle, PhidgetTemperatureSensorHandle as TemperatureSensorHandle,
    PhidgetTemperatureSensor_ThermocoupleType as ThermocoupleType,
};
use std::{mem, os::raw::c_void, ptr};

pub use ffi::PhidgetTemperatureSensor_ThermocoupleType_THERMOCOUPLE_TYPE_E as THERMOCOUPLE_TYPE_E;
pub use ffi::PhidgetTemperatureSensor_ThermocoupleType_THERMOCOUPLE_TYPE_J as THERMOCOUPLE_TYPE_J;
pub use ffi::PhidgetTemperatureSensor_ThermocoupleType_THERMOCOUPLE_TYPE_K as THERMOCOUPLE_TYPE_K;
pub use ffi::PhidgetTemperatureSensor_ThermocoupleType_THERMOCOUPLE_TYPE_T as THERMOCOUPLE_TYPE_T;

/// The function type for the safe Rust temperature change callback.
pub type TemperatureCallback = dyn Fn(&TemperatureSensor, f64) + Send + 'static;

/// Phidget temperature sensor
pub struct TemperatureSensor {
    // Handle to the sensor for the phidget22 library
    chan: TemperatureSensorHandle,
    // Double-boxed TemperatureCallback, if registered
    cb: Option<*mut c_void>,
    // Double-boxed attach callback, if registered
    attach_cb: Option<*mut c_void>,
    // Double-boxed detach callback, if registered
    detach_cb: Option<*mut c_void>,
}

impl TemperatureSensor {
    /// Create a new temperature sensor.
    pub fn new() -> Self {
        let mut chan: TemperatureSensorHandle = ptr::null_mut();
        unsafe {
            ffi::PhidgetTemperatureSensor_create(&mut chan);
        }
        Self::from(chan)
    }

    // Low-level, unsafe, callback for temperature change events.
    // The context is a double-boxed pointer the the safe Rust callback.
    unsafe extern "C" fn on_temperature_change(
        chan: TemperatureSensorHandle,
        ctx: *mut c_void,
        temperature: f64,
    ) {
        if !ctx.is_null() {
            let cb: &mut Box<TemperatureCallback> = &mut *(ctx as *mut _);
            let sensor = Self::from(chan);
            cb(&sensor, temperature);
            mem::forget(sensor);
        }
    }

    /// Get a reference to the underlying sensor handle
    pub fn as_channel(&self) -> &TemperatureSensorHandle {
        &self.chan
    }

    /// Read the current temperature
    pub fn temperature(&self) -> Result<f64> {
        let mut temperature = 0.0;
        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_getTemperature(self.chan, &mut temperature)
        })?;
        Ok(temperature)
    }

    /// Set a handler to receive temperature change callbacks.
    pub fn set_on_temperature_change_handler<F>(&mut self, cb: F) -> Result<()>
    where
        F: Fn(&TemperatureSensor, f64) + Send + 'static,
    {
        // 1st box is fat ptr, 2nd is regular pointer.
        let cb: Box<Box<TemperatureCallback>> = Box::new(Box::new(cb));
        let ctx = Box::into_raw(cb) as *mut c_void;
        self.cb = Some(ctx);

        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_setOnTemperatureChangeHandler(
                self.chan,
                Some(Self::on_temperature_change),
                ctx,
            )
        })
    }

    /// Sets a handler to receive attach callbacks
    pub fn set_on_attach_handler<F>(&mut self, cb: F) -> Result<()>
    where
        F: Fn(&GenericPhidget) + Send + 'static,
    {
        let ctx = crate::phidget::set_on_attach_handler(self, cb)?;
        self.attach_cb = Some(ctx);
        Ok(())
    }

    /// Sets a handler to receive detach callbacks
    pub fn set_on_detach_handler<F>(&mut self, cb: F) -> Result<()>
    where
        F: Fn(&GenericPhidget) + Send + 'static,
    {
        let ctx = crate::phidget::set_on_detach_handler(self, cb)?;
        self.detach_cb = Some(ctx);
        Ok(())
    }

    /// Set the thermocouple type (J = 1, K = 2, E = 3, T = 4).
    pub fn set_thermocouple_type<F>(&mut self, ty: ThermocoupleType) -> Result<()> {
        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_setThermocoupleType(self.chan, ty)
        })
    }

    /// Get the thermocouple type (J = 1, K = 2, E = 3, T = 4).
    pub fn get_thermocouple_type<F>(&mut self) -> Result<ThermocoupleType> {
        let mut thermocouple_type = 0;
        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_getThermocoupleType(self.chan, &mut thermocouple_type)
        })?;
        Ok(thermocouple_type)
    }

    /// Get the minimum temperature.
    pub fn get_min_temperature<F>(&mut self) -> Result<f64> {
        let mut min_temperature = 0.0;
        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_getMinTemperature(self.chan, &mut min_temperature)
        })?;
        Ok(min_temperature)
    }

    /// Get the maximum temperature.
    pub fn get_max_temperature<F>(&mut self) -> Result<f64> {
        let mut max_temperature = 0.0;
        ReturnCode::result(unsafe {
            ffi::PhidgetTemperatureSensor_getMaxTemperature(self.chan, &mut max_temperature)
        })?;
        Ok(max_temperature)
    }
}

impl Phidget for TemperatureSensor {
    fn as_handle(&mut self) -> PhidgetHandle {
        self.chan as PhidgetHandle
    }
}

unsafe impl Send for TemperatureSensor {}

impl Default for TemperatureSensor {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TemperatureSensorHandle> for TemperatureSensor {
    fn from(chan: TemperatureSensorHandle) -> Self {
        Self {
            chan,
            cb: None,
            attach_cb: None,
            detach_cb: None,
        }
    }
}

impl Drop for TemperatureSensor {
    fn drop(&mut self) {
        if let Ok(true) = self.is_open() {
            let _ = self.close();
        }
        unsafe {
            ffi::PhidgetTemperatureSensor_delete(&mut self.chan);
            crate::drop_cb::<TemperatureCallback>(self.cb.take());
            crate::drop_cb::<AttachCallback>(self.attach_cb.take());
            crate::drop_cb::<DetachCallback>(self.detach_cb.take());
        }
    }
}
