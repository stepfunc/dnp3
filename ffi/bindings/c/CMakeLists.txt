cmake_minimum_required(VERSION 3.8)

project(dnp3rs_c LANGUAGES C)

# Find dnp3rs
set(CMAKE_PREFIX_PATH ${CMAKE_CURRENT_LIST_DIR}/generated/cmake)
find_package(dnp3rs REQUIRED)

# Master example
add_executable(master_example master-example.c)
target_link_libraries(master_example PRIVATE dnp3rs)

# Outstation example
add_executable(outstation_example outstation-example.c)
target_link_libraries(outstation_example PRIVATE dnp3rs)

# Copy the DLL after build
add_custom_command(TARGET master_example POST_BUILD 
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3rs> $<TARGET_FILE_DIR:master_example>
)
add_custom_command(TARGET outstation_example POST_BUILD 
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3rs> $<TARGET_FILE_DIR:outstation_example>
)
