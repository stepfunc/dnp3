cmake_minimum_required(VERSION 3.8)

project(dnp3rs_c LANGUAGES C)

# Find dnp3rs
set(CMAKE_PREFIX_PATH ${CMAKE_CURRENT_LIST_DIR}/generated/cmake)
find_package(dnp3rs REQUIRED)

add_executable(dnp3rs_c main.c)
target_link_libraries(dnp3rs_c PRIVATE dnp3rs)

# Copy the DLL after build
add_custom_command(TARGET dnp3rs_c POST_BUILD 
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3rs> $<TARGET_FILE_DIR:dnp3rs_c>
)
