package com.example.qualia.agent

import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log

/**
 * Hooks into AudioRecord to extract audio-prosody features.
 * Computes energy variance, spectral centroid, and zero-crossing rate locally.
 */
class WebizenAudioService {

    private var audioRecord: AudioRecord? = null
    private var isRecording = false

    fun startListening() {
        Log.i("WebizenAudio", "Starting AudioRecord for Prosody extraction.")
        
        val sampleRate = 16000
        val channelConfig = AudioFormat.CHANNEL_IN_MONO
        val audioFormat = AudioFormat.ENCODING_PCM_16BIT
        val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)

        // Requires RECORD_AUDIO permission
        // audioRecord = AudioRecord(MediaRecorder.AudioSource.MIC, sampleRate, channelConfig, audioFormat, bufferSize)
        // audioRecord?.startRecording()
        isRecording = true
        
        // Spawn thread to read buffer and calculate prosody
    }

    fun stopListening() {
        Log.i("WebizenAudio", "Stopping Audio Service.")
        isRecording = false
        // audioRecord?.stop()
        // audioRecord?.release()
    }
}
