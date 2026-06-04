import re

filepath = r"C:\Projects\qualiaDB\docs\social\index.html"

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add 'Create Group Chat' icon to Chat List Header
header_old = """<a href="#" class="action p-r-10 pull-right link text-color">
                    <i class="pg-icon">more_horizontal</i>
                  </a>"""
header_new = """<a href="#" class="action p-r-10 pull-right link text-color" title="New Group Chat">
                    <i class="pg-icon">users</i>
                  </a>
                  <a href="#" class="action p-r-10 pull-right link text-color">
                    <i class="pg-icon">more_horizontal</i>
                  </a>"""
content = content.replace(header_old, header_new)

# 2. Add Group Chat List Item
group_chat_html = """
                <div class="list-view-group-container">
                  <div class="list-view-group-header text-uppercase">Groups</div>
                  <ul>
                    <li class="chat-user-list clearfix">
                      <a data-view-animation="push-parrallax" data-view-port="#chat" data-navigate="view" class="" href="#">
                        <span class="thumbnail-wrapper d32 circular bg-success" style="width:40px;">
                            <img width="20" height="20" alt="" src="assets/img/profiles/1x.jpg" style="position:absolute; left:0; top:0; border-radius:50%; border:2px solid #fff;">
                            <img width="20" height="20" alt="" src="assets/img/profiles/2x.jpg" style="position:absolute; right:0; top:0; border-radius:50%; border:2px solid #fff;">
                            <img width="20" height="20" alt="" src="assets/img/profiles/3x.jpg" style="position:absolute; bottom:0; left:10px; border-radius:50%; border:2px solid #fff;">
                        </span>
                        <p class="p-l-10 " style="margin-left:10px;">
                          <span class="text-color">Project Alpha Swarm</span>
                          <span class="block text-color hint-text fs-12">Ava: Looks good!</span>
                        </p>
                      </a>
                    </li>
                  </ul>
                </div>
"""
# Insert it right before the "a" group container
content = content.replace('<div class="list-view-group-container">\n                  <div class="list-view-group-header text-uppercase">\n                    a</div>', 
                          group_chat_html + '\n                <div class="list-view-group-container">\n                  <div class="list-view-group-header text-uppercase">\n                    a</div>')


# 3. Add Inline WebRTC Box
inline_webrtc = """
              <!-- BEGIN Inline WebRTC -->
              <div id="qualia-inline-webrtc" style="display: none; background: #000; position: relative; height: 180px; width: 100%;">
                  <video id="inline-remote-video" style="width: 100%; height: 100%; object-fit: cover;" autoplay playsinline muted></video>
                  <video id="inline-local-video" style="width: 60px; height: 80px; position: absolute; bottom: 10px; right: 10px; border: 2px solid white; border-radius: 6px; object-fit: cover; background: #333;" autoplay playsinline muted></video>
                  
                  <div style="position: absolute; top: 10px; right: 10px;">
                      <button class="btn btn-xs btn-primary" onclick="window.QualiaDB.toggleWebRTC('overlay')" title="Pop out"><i class="pg-icon">external_link</i></button>
                  </div>
                  <div style="position: absolute; bottom: 10px; left: 10px; display: flex; gap: 5px;">
                      <button class="btn btn-xs btn-default"><i class="pg-icon">mic</i></button>
                      <button class="btn btn-xs btn-danger" onclick="window.QualiaDB.toggleWebRTC('inline')"><i class="pg-icon">phone</i></button>
                  </div>
              </div>
              <!-- END Inline WebRTC -->
"""

# Insert right after <!-- END Header  -->
content = content.replace("<!-- END Header  -->", "<!-- END Header  -->\n" + inline_webrtc)

# We also need an icon in the Conversation Header to start the call
convo_header_old = """<a href="#" class="link text-color action p-r-10 pull-right ">
                    <i class="pg-icon">more_horizontal</i>
                  </a>"""
convo_header_new = """<a href="#" class="link text-color action p-r-10 pull-right" onclick="window.QualiaDB.toggleWebRTC('inline')">
                    <i class="pg-icon">video</i>
                  </a>
                  <a href="#" class="link text-color action p-r-10 pull-right ">
                    <i class="pg-icon">more_horizontal</i>
                  </a>"""
content = content.replace(convo_header_old, convo_header_new)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated index.html with Group Chat and Inline WebRTC.")
