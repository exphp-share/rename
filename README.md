# rename

Personal file renaming utility.  You use it like:

```
$ rename -g tests/snapshots/[]__anm12_[].snap tests/snapshots/[]_[].snap
mv 'tests/snapshots/bits_2_bits__anm12_anchor_ss_signature.snap'    'tests/snapshots/bits_2_bits_anchor_ss_signature.snap'
mv 'tests/snapshots/bits_2_bits__anm12_registers.snap'              'tests/snapshots/bits_2_bits_registers.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_duplicates.snap'      'tests/snapshots/bits_2_bits_sprite_duplicates.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_non_sequential.snap'  'tests/snapshots/bits_2_bits_sprite_non_sequential.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_script_args.snap'     'tests/snapshots/bits_2_bits_sprite_script_args.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_unset.snap'           'tests/snapshots/bits_2_bits_sprite_unset.snap'
mv 'tests/snapshots/bits_2_bits__anm12_unknown_signature.snap'      'tests/snapshots/bits_2_bits_unknown_signature.snap'
NOTICE: This was a DRY RUN!!!!!
        If you like the results, use the -D flag to DO IT!
```

It's a little bit... dumb right now, because I never bothered to revisit it.  Namely:

* The `-D` flag was never implemented, just pipe the output into `sh`.
* The `-g` flag is required for it to actually access the filesystem and search for existing files.  Without this, I think it takes on a "pure" behavior and only matches against the list of paths provided after the two patterns.  I cannot for the life of me remember why I designed it this way, and I don't think I ever used this feature!

Oh, some other features:

* You can change the command via e.g. `-x cp`.

* You can name the matchers to drop some or change up their order. e.g.

```
$ rename -g tests/snapshots/[a]__anm12_[b].snap [b]-[a]
mv 'tests/snapshots/bits_2_bits__anm12_anchor_ss_signature.snap'    'anchor_ss_signature-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_registers.snap'              'registers-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_duplicates.snap'      'sprite_duplicates-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_non_sequential.snap'  'sprite_non_sequential-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_script_args.snap'     'sprite_script_args-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_unset.snap'           'sprite_unset-bits_2_bits'
mv 'tests/snapshots/bits_2_bits__anm12_unknown_signature.snap'      'unknown_signature-bits_2_bits'
```

* You can let a group be a globstar by adding `:**`:

```
$ rename -g '[dir:**]/[]__anm12_[b].snap' [dir]/[b].snap
mv 'tests/snapshots/bits_2_bits__anm12_anchor_ss_signature.snap'    'tests/snapshots/anchor_ss_signature.snap'
mv 'tests/snapshots/bits_2_bits__anm12_registers.snap'              'tests/snapshots/registers.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_duplicates.snap'      'tests/snapshots/sprite_duplicates.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_non_sequential.snap'  'tests/snapshots/sprite_non_sequential.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_script_args.snap'     'tests/snapshots/sprite_script_args.snap'
mv 'tests/snapshots/bits_2_bits__anm12_sprite_unset.snap'           'tests/snapshots/sprite_unset.snap'
mv 'tests/snapshots/bits_2_bits__anm12_unknown_signature.snap'      'tests/snapshots/unknown_signature.snap'
```
