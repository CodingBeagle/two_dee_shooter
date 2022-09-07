param ($project_root_directory)

Copy-Item -Path "$($project_root_directory)\beagle_glfw\resources\glfw3.dll" -Destination "$($project_root_directory)\\target\\debug" -Force

Start-Process "cargo" -ArgumentList "build" -PassThru -NoNewWindow