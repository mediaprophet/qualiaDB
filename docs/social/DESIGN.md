# Webizen UI Design & Architecture Notes

## Overview
This document outlines the design decisions made for the Webizen Social App UI, specifically regarding its integration with the offline-first QualiaDB engine and W3C Solid networks.

## 1. Decentralized Identity & Schema Mapping (qualia-solid-bridge)
To maintain backwards compatibility with existing Web2/W3C Solid systems while leveraging QualiaDB's native zero-allocation architecture:
- **WebID Standard**: The frontend UI utilizes standard W3C Solid `WebID` URLs (e.g., `https://user.solidcommunity.net/profile/card#me`) and `did:git` identifiers.
- **Allocation Firewall**: Because the QualiaDB core engine relies strictly on 64-bit Quin vectors and bit-packed memory, the frontend (via `qualia-integration.js`) serves as the translation bridge. Standard JSON HTTP/Solid data is ingested, mapped to UI elements, and hashed into integer Quins before being passed to the `qualia_query` JSON-RPC WebSocket.
- **Group Chats**: Group video architecture relies on the local QualiaDB daemon acting as an SFU (Selective Forwarding Unit) / Swarm node, routing WebRTC streams directly without a central cloud server.

## 2. WebRTC & Neurosymbolic Chat Integration
- **Inline + Pop-out Model**: WebRTC video feeds are embedded directly inline within the active chat thread for contextual awareness. A "pop-out" functionality allows the video to detach into a floating overlay (picture-in-picture) so users can navigate to other parts of the Webizen app (like Qmail or Dashboard) while maintaining the call.
- **Offline Transcoding**: Speech-To-Text and translation are handled entirely on the client side using the browser's native `SpeechRecognition` API and `Transformers.js`. This guarantees that no audio data leaks to third-party cloud translation services, aligning with QualiaDB's privacy-first ethos.
