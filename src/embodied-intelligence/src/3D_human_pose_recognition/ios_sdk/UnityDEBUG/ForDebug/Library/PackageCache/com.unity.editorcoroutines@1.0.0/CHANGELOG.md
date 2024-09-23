# Changelog
All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2020-04-01
### Changed
 - Package has been verified for 2020.2


## [0.1.0-preview.2] - 2020-03-17
### Changed
 - Lowered data size for the EditorWaitForSeconds class by half.

## [0.1.0-preview.1] - 2020-01-08
### Added
 - Added support for AsyncOperation subclasses.
### Changed
 - Fixed unstable test.

## [0.0.2-preview.1] - 2019-01-25
### Changed
 - Fixed a compilation issue caused by using the 'default' literal.

## [0.0.1-preview.5] - 2019-01-14
### Changed
 - Updated Readme.md.
 - Added unified yield statement processor.
 - Added stack based processing of nested yield statements.
 - Updated tests.
 - Lowered memory footprint of editor coroutine instances.

### Removed
 - Removed recursive handling of nested yield statements.
 - Removed specialized yield statement processors.

## [0.0.1-preview.4] - 2018-12-7
### Added
 - API documentation.

### Changed
 - Fixed line endings for the EditorCourtineTests.cs source file.

## [0.0.1-preview.3] - 2018-10-11
### Changed
 - Updated LICENSE.md.
 - Updated manifest to reflect correct minimum supported version.

## [0.0.1-preview.2] - 2018-10-11
### Added 
 - Added stub documentation via com.unity.editorcoroutines.md.

## [0.0.1-preview.1] - 2018-10-10
### Added
 - Added nesting support for editor coroutines.
 - Added abitrary enumerator support for editor coroutines.
 - Created specialized EditorWaitForSeconds class with access to it's wait time ( same behavior as WaitForSeconds).


### This is the first release of *Unity Package Editor Coroutines*.
 Source code release of the Editor Coroutines package, with no added documentation or stripping of default Package Creation Kit files.

