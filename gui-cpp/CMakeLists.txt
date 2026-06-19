cmake_minimum_required(VERSION 3.15)
project(kto_gui LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "kto-gui is a Win32 app and only builds on Windows.")
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

enable_language(RC)

add_executable(kto-gui WIN32 main.cpp app.rc)
target_link_libraries(kto-gui PRIVATE comctl32 advapi32 user32 gdi32 shell32)
set_target_properties(kto-gui PROPERTIES OUTPUT_NAME "kto-gui")

if(MSVC)
    # app.rc already embeds the manifest, so stop the linker from adding a
    # second one (otherwise CVT1100 "duplicate resource" / LNK1123).
    target_link_options(kto-gui PRIVATE /MANIFEST:NO)
endif()
