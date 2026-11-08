# Why the Far Lands looked like a smooth wall, and what changed

The port's logic matches the Java mod (density wrapper, coordinate adjust, no wrap). The difference is
number size. Past 2^31 in noise space an octave's cell stops moving while the position inside it keeps
growing, so the fade polynomial reaches 1e21 a dozen blocks past the edge and 1e40+ further out. Classic
Minecraft summed this in doubles; the sign of the huge value is what draws the stripes and walls.
Minecraft 26.x noise, and Horizon's copy of it, work in f32 (max 3.4e38) with f32 buffers, so a few
thousand blocks past the edge every sample is infinity or NaN and the density collapses to a flat wall.

`farlands_noise.rs` now computes any sample whose X or Z cell has saturated in f64 (`classic_get3`, and
the far-column branch of `add_to_volume`), with Java's `Mth.floor` wrap for the far negative side, and
scales its size down to about 1e12 (`classic_scale`): finite in f32, far above ordinary noise (about 1),
same sign, same proportions between the noises of one position. Positions inside the normal range take
the old f32 path unchanged.
