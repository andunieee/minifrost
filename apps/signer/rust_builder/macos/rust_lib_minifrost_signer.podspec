#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
#
Pod::Spec.new do |s|
  s.name             = 'rust_lib_minifrost_signer'
  s.version          = '0.0.1'
  s.summary          = 'Rust library for the Minifrost signer.'
  s.description      = <<-DESC
The Minifrost signer's FROST protocol code, built by Cargokit.
                       DESC
  s.homepage         = 'https://github.com/frostsnap/frostsnap'
  s.license          = { :type => 'MIT OR Apache-2.0' }
  s.author           = { 'Minifrost' => 'dev@minifrost.dev' }

  # Classes contains a forwarder C file so that the C sources can be shared among all target
  # platforms; the actual Rust library is built by the script phase below.
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'

  s.platform = :osx, '12.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  s.swift_version = '5.0'

  s.script_phase = {
    :name => 'Build Rust library',
    # First argument is the relative path to the Rust crate, second is the library name.
    :script => 'sh "$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh" ../../../../crates/minifrost_signer minifrost_signer',
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    # Let Xcode know that the static library referenced in -force_load below is created by this
    # build step.
    :output_files => ["${BUILT_PRODUCTS_DIR}/librust_lib_minifrost_signer.a"],
  }
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    # Flutter.framework does not contain a i386 slice.
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/librust_lib_minifrost_signer.a',
  }
end
