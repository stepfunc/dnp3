cmake_minimum_required(VERSION 3.11)

project(dnp3_c LANGUAGES C)

set(DNP3_VERSION 0.9.1)

# Determine the architecture
if(WIN32 AND CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|amd64|AMD64" AND CMAKE_SIZEOF_VOID_P EQUAL 8)
    set(DNP3_ARCH "x86_64-pc-windows-msvc")
elseif(UNIX AND CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|amd64|AMD64")
    set(DNP3_ARCH "x86_64-unknown-linux-gnu")
elseif(UNIX AND CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64")
    set(DNP3_ARCH "aarch64-unknown-linux-gnu")
else()
    message(FATAL_ERROR "target architecture not supported by this CMake file")
endif()

# Find the DNP3 package
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/generated/${DNP3_ARCH})
    # Use the locally built library
    find_package(dnp3 REQUIRED HINTS ${CMAKE_CURRENT_LIST_DIR}/generated/${DNP3_ARCH}/cmake)
else()
    # Download the library from GitHub
    include(FetchContent)
    FetchContent_Declare(
        dnp3
        URL https://github.com/stepfunc/dnp3/releases/download/${DNP3_VERSION}/dnp3-${DNP3_VERSION}-${DNP3_ARCH}.zip
    )

    FetchContent_GetProperties(dnp3)
    if(NOT dnp3_POPULATED)
        FetchContent_Populate(dnp3)
        find_package(dnp3 REQUIRED HINTS ${dnp3_SOURCE_DIR})
    endif()
endif()

# Master example
add_executable(master_example master_example.c)
target_link_libraries(master_example PRIVATE dnp3)

# Outstation example
add_executable(outstation_example outstation_example.c)
target_link_libraries(outstation_example PRIVATE dnp3)

# Master TLS example
add_executable(master_tls_example master_tls_example.c)
target_link_libraries(master_tls_example PRIVATE dnp3)

# Outstation TLS example
add_executable(outstation_tls_example outstation_tls_example.c)
target_link_libraries(outstation_tls_example PRIVATE dnp3)

# Copy the DLL after build
add_custom_command(TARGET master_example POST_BUILD 
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3> $<TARGET_FILE_DIR:master_example>
)
add_custom_command(TARGET outstation_example POST_BUILD 
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:dnp3> $<TARGET_FILE_DIR:outstation_example>
)
