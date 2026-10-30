[property-package]
name = PengRobinson

[components]
component = WATER|Water|7732-18-5|H2O
component = METHANOL|Methanol|67-56-1|CH4O
component = ETHANOL|Ethanol|64-17-5|C2H6O

[phases]
phase = Vapor
phase = Liquid

[single-phase-properties]
property = temperature
property = pressure
property = enthalpy
property = molarVolume
property = viscosity

[two-phase-properties]
property = vaporFraction
property = equilibriumTemperature

