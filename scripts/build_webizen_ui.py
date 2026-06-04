import os
import re
import glob

target_dir = r"C:\Projects\qualiaDB\docs\social"

# The new sidebar menu HTML
new_sidebar = """<div class="sidebar-menu">
        <ul class="menu-items">
          <li class="m-t-10">
            <a href="index.html" class="detailed">
              <span class="title">Dashboard</span>
              <span class="details">System Stats</span>
            </a>
            <span class="icon-thumbnail"><i data-feather="shield"></i></span>
          </li>
          <li class="">
            <a href="qmail.html" class="detailed">
              <span class="title">Qmail</span>
              <span class="details">2 New Messages</span>
            </a>
            <span class="icon-thumbnail"><i data-feather="mail"></i></span>
          </li>
          <li class="">
            <a href="social.html"><span class="title">Social</span></a>
            <span class="icon-thumbnail"><i data-feather="users"></i></span>
          </li>
          <li class="">
            <a href="javascript:;"><span class="title">Cooperative Projects</span>
            <span class=" arrow"></span></a>
            <span class="icon-thumbnail"><i data-feather="briefcase"></i></span>
            <ul class="sub-menu">
              <li class=""><a href="cooperative.html">Hub</a><span class="icon-thumbnail">H</span></li>
              <li class=""><a href="project-detail.html">Project Detail</a><span class="icon-thumbnail">PD</span></li>
              <li class=""><a href="analytics.html">Analytics</a><span class="icon-thumbnail">A</span></li>
              <li class=""><a href="kanban.html">Kanban</a><span class="icon-thumbnail">K</span></li>
              <li class=""><a href="roadmap.html">Roadmap</a><span class="icon-thumbnail">R</span></li>
              <li class=""><a href="issues.html">Issues</a><span class="icon-thumbnail">I</span></li>
              <li class=""><a href="project-assets.html">Assets</a><span class="icon-thumbnail">PA</span></li>
              <li class=""><a href="budgets.html">Budgets</a><span class="icon-thumbnail">B</span></li>
              <li class=""><a href="canvases.html">Canvases</a><span class="icon-thumbnail">C</span></li>
              <li class=""><a href="contracts.html">Contracts & Claims</a><span class="icon-thumbnail">CC</span></li>
              <li class=""><a href="contribute.html">Contribute & Sponsor</a><span class="icon-thumbnail">CS</span></li>
              <li class=""><a href="docuquin-pipeline.html">DocuQuin</a><span class="icon-thumbnail">DQ</span></li>
              <li class=""><a href="library.html">Library</a><span class="icon-thumbnail">L</span></li>
            </ul>
          </li>
          <li class="">
            <a href="calendar.html"><span class="title">Calendar</span></a>
            <span class="icon-thumbnail"><i data-feather="calendar"></i></span>
          </li>
          <li class="">
            <a href="notifications.html"><span class="title">Chat / Alerts</span></a>
            <span class="icon-thumbnail"><i data-feather="bell"></i></span>
          </li>
          <li class="">
            <a href="timeline.html"><span class="title">Timeline</span></a>
            <span class="icon-thumbnail"><i data-feather="clock"></i></span>
          </li>
          <li class="">
            <a href="maps.html"><span class="title">Maps</span></a>
            <span class="icon-thumbnail"><i data-feather="map"></i></span>
          </li>
          <li class="">
            <a href="#" onclick="window.QualiaDB.toggleWebRTC(); return false;">
              <span class="title">Voice / Video Call</span>
            </a>
            <span class="icon-thumbnail"><i data-feather="video"></i></span>
          </li>
        </ul>
        <div class="clearfix"></div>
      </div>
      <!-- END SIDEBAR MENU -->"""

webrtc_overlay = """
    <!-- WebRTC Overlay UI -->
    <div id="qualia-webrtc-overlay" style="display: none; position: fixed; bottom: 20px; right: 20px; width: 350px; background: #fff; border-radius: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.2); z-index: 9999; overflow: hidden; border: 1px solid #eaeaea;">
        <div style="background: #2b303b; color: white; padding: 10px 15px; display: flex; justify-content: space-between; align-items: center; cursor: pointer;" onclick="window.QualiaDB.toggleWebRTC()">
            <h5 style="margin: 0; color: white; font-size: 14px;"><i class="pg-icon m-r-5">video</i> Secure WebRTC Call</h5>
            <span id="webrtc-status-indicator" style="font-size: 12px; color: #48b0f7;">Initializing...</span>
        </div>
        <div style="height: 200px; background: #000; position: relative;">
            <video id="webrtc-remote-video" style="width: 100%; height: 100%; object-fit: cover;" autoplay playsinline muted></video>
            <video id="webrtc-local-video" style="width: 80px; height: 100px; position: absolute; bottom: 10px; right: 10px; border: 2px solid white; border-radius: 8px; object-fit: cover; background: #333;" autoplay playsinline muted></video>
        </div>
        <div style="padding: 15px; background: #fafafa; border-top: 1px solid #eee;">
            <div id="qualia-transcript-box" style="height: 120px; overflow-y: auto; font-size: 13px; color: #555; margin-bottom: 10px; padding: 5px; background: #fff; border: 1px solid #f0f0f0; border-radius: 4px;">
                <em style="color: #999;">Waiting for connection...</em>
            </div>
            <div style="display: flex; justify-content: space-around;">
                <button class="btn btn-default btn-cons"><i class="pg-icon">mic</i></button>
                <button class="btn btn-default btn-cons"><i class="pg-icon">video</i></button>
                <button class="btn btn-danger btn-cons" onclick="window.QualiaDB.toggleWebRTC()"><i class="pg-icon">phone</i></button>
            </div>
        </div>
    </div>
"""

sidebar_regex = re.compile(r'<div class="sidebar-menu">.*?<!-- END SIDEBAR MENU -->', re.DOTALL)
body_close_regex = re.compile(r'</body>')
logo_regex = re.compile(r'<img[^>]*?assets/img/logo[^>]*?>')
title_regex = re.compile(r'<title>.*?</title>')

def process_html_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Replace sidebar block entirely
    content = sidebar_regex.sub(new_sidebar, content)
    
    # Renames
    content = content.replace('email.html', 'qmail.html')
    content = content.replace('email_compose.html', 'qmail_compose.html')
    content = content.replace('vector_map.html', 'maps.html')
    
    # Replace Email text with Qmail safely inside elements
    content = content.replace(">Email<", ">Qmail<")
    content = content.replace("> Email<", "> Qmail<")
    content = content.replace(" Email ", " Qmail ")

    # Remove branding
    content = logo_regex.sub('<h4 style="margin: 0; padding-top: 5px; font-weight: bold; color: #fff;">Webizen</h4>', content)
    # Fix the brand inline color so it's visible on white headers
    content = content.replace('<div class="brand inline">', '<div class="brand inline" style="color: #333 !important;">')
    content = content.replace('color: #fff;', 'color: inherit;') # generic fix for the h4
    content = title_regex.sub('<title>Webizen Qualia App</title>', content)
    
    # Inject overlay
    script_and_overlay = webrtc_overlay + '\n    <script type="module" src="assets/js/qualia-integration.js"></script>\n  </body>'
    
    # Remove old injected script just in case
    content = content.replace('<script type="module" src="assets/js/qualia-integration.js"></script>', '')
    
    content = body_close_regex.sub(script_and_overlay, content)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

# Rename the vector_map file first
old_map = os.path.join(target_dir, "vector_map.html")
new_map = os.path.join(target_dir, "maps.html")
if os.path.exists(old_map):
    os.rename(old_map, new_map)

for root, _, files in os.walk(target_dir):
    for file in files:
        if file.endswith('.html'):
            filepath = os.path.join(root, file)
            print(f"Processing {filepath}...")
            process_html_file(filepath)
            
print("Done fixing HTML files.")
