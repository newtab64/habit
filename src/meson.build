cargo = find_program('cargo', required: true)

cargo_options = [
  '--manifest-path', meson.project_source_root() / 'Cargo.toml',
]

if get_option('buildtype') == 'release'
  cargo_options += ['--release']
  rust_target_dir = 'release'
else
  rust_target_dir = 'debug'
endif

cargo_options += ['--features', 'ui']

custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'habit',
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'sh', '-c',
    '@0@ build @1@ && cp "@2@" "@OUTPUT@"'.format(
      cargo.full_path(),
      ' '.join(cargo_options),
      meson.project_source_root() / 'target' / rust_target_dir / 'habit',
    ),
  ],
)
