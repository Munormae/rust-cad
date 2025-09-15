#include "ifc_bridge.hpp"

namespace ifcbridge {

// тут тоже rust::Str -> const char*
std::unique_ptr<FileRaw> import_ifc(const char* /*path*/) noexcept {
    auto out = std::make_unique<FileRaw>();

    out->pts_storage_.reserve(4);
    out->pts_storage_.push_back({0,0});
    out->pts_storage_.push_back({1,0});
    out->pts_storage_.push_back({1,1});
    out->pts_storage_.push_back({0,1});

    ExtrusionRaw ex{};
    ex.profile.pts = out->pts_storage_.data();
    ex.profile.len = 4;
    ex.height = 1.0;
    for (int i = 0; i < 16; ++i) ex.xform[i] = (i % 5 == 0) ? 1.0 : 0.0;

    out->extrusions_.push_back(ex);
    return out;
}

} // namespace ifcbridge
