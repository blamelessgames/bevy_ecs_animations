# Changes


### 0.20.1
This release includes a few new features to enhance the functionality and ease the use of the crate, as well
as some general improvements to the code and documentation
- new attribute macro to define the `Animation` implementation
- PersistentTick, to avoid archetype changes if desired
- Added a per-animation type SystemSet (AnimationSystems), exposed for ordering
- some internal improvements to ticking
- doc fixes
- updated MSRV to 1.97.1, missed from the bevy update

### 0.20.0
- updated to bevy 0.20
- removed deprecated code
- new "cascade" example, thanks @bokubeam!

### 0.19.2
- deprecated the existing API and made it much simpler and amenable to a derive macro

### 0.19.1
- Fixed the way t values are calculated, in my zeal to release I skipped test cases that later occurred to me, which let a dumb bug through where dt wasn't accumulated til the next frame

### 0.19.0 
- initial release