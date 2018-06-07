#!/bin/sh
gst-launch-1.0 -v udpsrc port=10000 ! application/x-rtp,media=video,encoding-name=H264 ! queue ! rtph264depay ! avdec_h264 ! videoconvert ! autovideosink
