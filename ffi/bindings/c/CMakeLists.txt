cmake_minimum_required(VERSION 3.8)

project(dnp3_c LANGUAGES C)

# Find dnp3
if(WIN32)
    set(CMAKE_PREFIX_PATH ${CMAKE_CURRENT_LIST_DIR}/generated/x86_64-pc-windows-msvc/cmake)
elseif(UNIX)
    set(CMAKE_PREFIX_PATH ${CMAKE_CURRENT_LIST_DIR}/generated/x86_64-unknown-linux-gnu/cmake)
endif()
find_package(dnp3 REQUIRED)

# You probably don't need do this, just pick either the dynamic or the static
# one if you are on Linux. Here we do CMake trickery to make it work on the CI.
if(TARGET dnp3)
    set(dnp3_target dnp3)
else()
    set(dnp3_target dnp3_static)
endif()

# Master example
add_executable(master_example master_example.c)
target_link_libraries(master_example PRIVATE ${dnp3_target})

# Outstation example
add_executable(outstation_example outstation_example.c)
target_link_libraries(outstation_example PRIVATE ${dnp3_target})

if(${dnp3_target} STREQUAL "dnp3")
    # Copy the DLL after build
    add_custom_command(TARGET master_example POST_BUILD 
        COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3> $<TARGET_FILE_DIR:master_example>
    )
    add_custom_command(TARGET outstation_example POST_BUILD 
        COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3> $<TARGET_FILE_DIR:outstation_example>
    )
endif()
