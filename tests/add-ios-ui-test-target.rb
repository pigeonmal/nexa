require 'xcodeproj'

project_path = ARGV.fetch(0)
test_source = ARGV.fetch(1)
target_name = ARGV.fetch(2)

project = Xcodeproj::Project.open(project_path)
project.root_object.project_dir_path = '.'
app_target = project.targets.find do |target|
  target.product_type == 'com.apple.product-type.application'
end
abort 'could not find the generated iOS app target' unless app_target
abort "test target already exists: #{target_name}" if project.targets.any? { |target| target.name == target_name }

test_target = project.new_target(:ui_test_bundle, target_name, :ios, '16.0')
test_target.add_dependency(app_target)
app_bundle_id = app_target.build_configurations.first.build_settings.fetch('PRODUCT_BUNDLE_IDENTIFIER')
app_deployment_target = app_target.build_configurations.first.build_settings.fetch('IPHONEOS_DEPLOYMENT_TARGET', '16.0')
test_target.build_configurations.each do |configuration|
  configuration.build_settings.merge!(
    'CODE_SIGNING_ALLOWED' => 'NO',
    'GENERATE_INFOPLIST_FILE' => 'YES',
    'IPHONEOS_DEPLOYMENT_TARGET' => app_deployment_target,
    'PRODUCT_BUNDLE_IDENTIFIER' => "#{app_bundle_id}.uitests",
    'PRODUCT_NAME' => target_name,
    'SUPPORTED_PLATFORMS' => 'iphoneos iphonesimulator',
    'SWIFT_VERSION' => '6.0',
    'TARGETED_DEVICE_FAMILY' => '1,2',
    'TEST_TARGET_BUNDLE_IDENTIFIER' => app_bundle_id,
    'TEST_TARGET_NAME' => app_target.name,
  )
end

project.main_group.new_file(test_source)
test_file = project.files.find { |file| file.path == test_source }
abort "could not register UI test source: #{test_source}" unless test_file
test_target.add_file_references([test_file])
project.save

scheme = Xcodeproj::XCScheme.new
scheme.add_build_target(app_target)
scheme.add_test_target(test_target)
scheme.set_launch_target(app_target)
scheme.save_as(project_path, target_name)
puts "#{app_bundle_id}.uitests.xctrunner"
