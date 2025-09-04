cmake_minimum_required(VERSION 3.20)

project(BunkerMinerClient 
    VERSION 0.1.0 
    LANGUAGES CXX
    DESCRIPTION "BUNKER MINER - Desktop client application"
)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Set build type if not specified
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# Compiler-specific options
if(MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W4 /WX")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /Od /Zi")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /O2 /DNDEBUG")
else()
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Werror")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -O0")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O3 -DNDEBUG")
endif()

# Find required packages
find_package(Qt6 REQUIRED COMPONENTS Core Widgets Network)
find_package(Protobuf REQUIRED)
find_package(gRPC REQUIRED)

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/src)
include_directories(${CMAKE_CURRENT_BINARY_DIR}) # For generated files

# Protocol Buffers and gRPC stub generation
set(PROTO_PATH ${CMAKE_SOURCE_DIR}/../protos)
file(GLOB PROTO_FILES ${PROTO_PATH}/*.proto)

# Generated source files will be added here once protos are defined
set(PROTO_GENERATED_DIR ${CMAKE_CURRENT_BINARY_DIR}/generated)
file(MAKE_DIRECTORY ${PROTO_GENERATED_DIR})

# Source files
set(SOURCES
    src/main.cpp
    src/MainWindow.cpp
    src/DaemonGrpcClient.cpp
    # Additional source files will be added in Phase 2
)

set(HEADERS
    src/MainWindow.h
    src/DaemonGrpcClient.h
    # Additional headers will be added in Phase 2
)

# UI files (Qt Designer files)
set(UI_FILES
    ui/MainWindow.ui
    # Additional UI files will be added in Phase 2
)

# Qt MOC (Meta-Object Compiler) processing
qt6_wrap_cpp(MOC_SOURCES ${HEADERS})

# Qt UIC (User Interface Compiler) processing
qt6_wrap_ui(UI_HEADERS ${UI_FILES})

# Create executable
add_executable(${PROJECT_NAME}
    ${SOURCES}
    ${HEADERS}
    ${MOC_SOURCES}
    ${UI_HEADERS}
    # Protocol buffer generated files will be added here
)

# Link libraries
target_link_libraries(${PROJECT_NAME}
    Qt6::Core
    Qt6::Widgets
    Qt6::Network
    protobuf::libprotobuf
    gRPC::grpc++
)

# Set target properties
set_target_properties(${PROJECT_NAME} PROPERTIES
    OUTPUT_NAME "bunker-miner-client"
    WIN32_EXECUTABLE TRUE
)

# Platform-specific settings
if(WIN32)
    # Windows-specific settings
    set_target_properties(${PROJECT_NAME} PROPERTIES
        LINK_FLAGS "/SUBSYSTEM:WINDOWS"
    )
endif()

if(UNIX AND NOT APPLE)
    # Linux-specific settings
    target_link_libraries(${PROJECT_NAME} pthread)
endif()

# Install rules
install(TARGETS ${PROJECT_NAME}
    BUNDLE DESTINATION .
    RUNTIME DESTINATION bin
)

# Qt deployment (for packaging)
if(Qt6_FOUND AND WIN32 AND TARGET Qt6::qmake AND NOT TARGET Qt6::windeployqt)
    get_target_property(_qt6_qmake_location Qt6::qmake IMPORTED_LOCATION)
    execute_process(
        COMMAND "${_qt6_qmake_location}" -query QT_INSTALL_PREFIX
        RESULT_VARIABLE return_code
        OUTPUT_VARIABLE qt6_install_prefix
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    
    set(imported_location "${qt6_install_prefix}/bin/windeployqt.exe")
    
    if(EXISTS ${imported_location})
        add_executable(Qt6::windeployqt IMPORTED)
        set_target_properties(Qt6::windeployqt PROPERTIES IMPORTED_LOCATION ${imported_location})
    endif()
endif()

# Development stubs (Phase 0.1)
# Create basic source files if they don't exist
if(NOT EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/src/main.cpp)
    configure_file(
        ${CMAKE_CURRENT_SOURCE_DIR}/templates/main.cpp.in
        ${CMAKE_CURRENT_SOURCE_DIR}/src/main.cpp
    )
endif()

# Enable testing
enable_testing()

# Add subdirectories for tests (when implemented in Phase 2)
# add_subdirectory(tests)

# Print configuration summary
message(STATUS "BunkerMinerClient Configuration Summary:")
message(STATUS "  Version: ${PROJECT_VERSION}")
message(STATUS "  Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  C++ standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Qt6 version: ${Qt6_VERSION}")

# Development phase marker
message(STATUS "Phase 0.1: Basic CMake structure initialized")
message(STATUS "Full client implementation scheduled for Phase 2")