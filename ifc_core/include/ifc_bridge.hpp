#pragma once
#include <cstdint>
#include <memory>
#include <vector>

namespace ifcbridge {

struct Pt2 { double x; double y; };

struct ProfileRaw {
    const Pt2* pts;
    int32_t    len;
};

struct ExtrusionRaw {
    ProfileRaw profile;
    double     height;
    double     xform[16];
};

class FileRaw {
public:
    FileRaw() = default;
    std::vector<ExtrusionRaw> extrusions_;
    std::vector<Pt2>          pts_storage_;
};

// !!! тут поменяли rust::Str -> const char*
std::unique_ptr<FileRaw> import_ifc(const char* path) noexcept;

inline const ExtrusionRaw* extrusions_ptr(const FileRaw& f) noexcept {
    return f.extrusions_.empty() ? nullptr : f.extrusions_.data();
}
inline size_t extrusions_len(const FileRaw& f) noexcept {
    return f.extrusions_.size();
}

} // namespace ifcbridge
