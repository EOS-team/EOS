# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [1.0.3] - 2020-06-21

### Bug Fixes

- Fixed `PackageSettingsRepository` dirtying the settings file when no changes are present.

## [1.0.2] - 2020-02-26

### Bug Fixes

- Fixed obsolete API use in Unity 2019.3.

### Changes

- Update Yamato configuration.

## [1.0.1] - 2019-11-25

### Changes

- Make sure version control integration grants write access before trying to save package settings.

### Bug Fixes

- Fixed samples not compiling with Unity 2019.3.
- Fix package settings repo potentially initializing with a null dictionary.

## [1.0.0] - 2019-04-03

### Bug Fixes

- Fixed compile errors on Unity 2018.4.

## [0.1.0-preview.8] - 2019-03-29

### Features

- Support saving multiple settings repositories within a project

### Changes

- Rename `ProjectSettingsRepository` -> `PackageSettingsRepository`.
- Update readme with a complete code example.
- Add additional documentation and unit tests.
- Setting repositories now have names.

### Bug Fixes

- Fixed missing gear icon in Settings Provider implementation.

## [0.1.0-preview.4] - 2019-02-28

- Package configuration update.

## [0.1.0-preview.3] - 2019-02-27

- Small code update in sample.

## [0.1.0-preview.2] - 2019-02-22

- Rebuild meta files.

## [0.1.0-preview.1] - 2019-02-01

- Move samples outside of main package.

## [0.1.0-preview.0] - 2018-10-08

This is the first release of *Unity Package Settings Manager*.