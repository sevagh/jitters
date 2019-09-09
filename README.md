# jitters

jitters is an incomplete RTP jitter buffer demo, written in Rust. What follows is a description of the various subcomponents of jitters based on the directories.

### src

rtp.rs contains some structs for working with a very lean subset of RTP:

* RTP without extensions, custom payload types, or csrcs
* The only supported payload types are 44100Hz L16 uncompressed audio mono and stereo. This means resampling to 44100 is required.
* The marker bit of the header is used to mark the end of a stream (more as a personal convenience, so I know I can start playing the audio stream)
* The initial sequence and timestamp are selected randomly, as recommended by the RFC
    * The sequence is incremented by 1, the timestamp is incremented by the number of samples sent in the packet (i.e. `JITTERS_MAX_PACKET_SIZE - size_of::<RtpHeader>() = 1400 - 12 = 1388`). The timestamp can be converted to `ms` by the receiver using the sample rate

I wrote the bulk of the code from [the original RFC](https://tools.ietf.org/html/rfc3550). I've also implemented waveform substitution for [packet loss concealment](https://en.wikipedia.org/wiki/Packet_loss_concealment#PLC_techniques).

### examples

#### wav_sender

`wav_sender.rs` uses the [hound](https://github.com/ruuda/hound/) crate to convert a WAV file to an outgoing RTP stream. The stream is recognized by wireshark and ffplay.

In one command prompt, run `ffplay`:

```
sevagh:jitters $ ffplay -hide_banner rtp://@127.0.0.1:1337
[rtp @ 0x7f011c000b80] Guessing on RTP content - if not received properly you need an SDP file describing it
Input #0, rtp, from 'rtp://@127.0.0.1:1337':
  Duration: N/A, start: 0.000000, bitrate: 1411 kb/s
    Stream #0:0: Audio: pcm_s16be, 44100 Hz, 2 channels, s16, 1411 kb/s
[rtp @ 0x7f011c000b80] jitter buffer full=    0KB sq=    0B f=0/0
[rtp @ 0x7f011c000b80] RTP: missed 443 packets
[rtp @ 0x7f011c000b80] jitter buffer full=    0KB sq=    0B f=0/0
[rtp @ 0x7f011c000b80] RTP: missed 20 packets
[rtp @ 0x7f011c000b80] jitter buffer full=    0KB sq=    0B f=0/0
[rtp @ 0x7f011c000b80] RTP: missed 1 packets
  63.77 M-A:  0.000 fd=   0 aq=  584KB vq=    0KB sq=    0B f=0/0
```

In another, run the wav_sender example:

```
sevagh:jitters $ cargo run --example wav_sender '127.0.0.1:13337' '127.0.0.1:1337' 188692__arseniiv__pianoa-100bpm.wav
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/examples/wav_sender '127.0.0.1:13337' '127.0.0.1:1337' 188692__arseniiv__pianoa-100bpm.wav`
Sent samples at timestamp 0.0ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 7.8684807256235825ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 15.736961451247165ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 23.605442176870746ms with RTP over UDP to 127.0.0.1:1337
```

![ffplay](.github/ffplay.png)

### wav_receiver, wav_jitter_receiver

Similar to the above send/receive test with `ffplay`, run the receiver examples to listen to the WAV file:


```
sevagh:jitters $ cargo run --example wav_jitter_receiver '127.0.0.1:1337'
...
Received 1400 bytes from 127.0.0.1:13337
Stream ended - playing audio...
Corrected 0 out-of-order packets
Yielding audio slice for sequence 0, timestamp 0.0ms
Yielding audio slice for sequence 1, timestamp 7.8684807256235825ms
Yielding audio slice for sequence 2, timestamp 15.736961451247165ms
Yielding audio slice for sequence 3, timestamp 23.60544217687075ms
...
Yielding audio slice for sequence 2437, timestamp 19175.48752834467ms
Yielding audio slice for sequence 2438, timestamp 19183.356009070296ms
Yielding audio slice for sequence 2439, timestamp 19191.224489795917ms
audio done, exiting program
```

In another, run the wav_sender example:

```
sevagh:jitters $ cargo run --example wav_sender '127.0.0.1:13337' '127.0.0.1:1337' 188692__arseniiv__pianoa-100bpm.wav
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/examples/wav_sender '127.0.0.1:13337' '127.0.0.1:1337' 188692__arseniiv__pianoa-100bpm.wav`
Sent samples at timestamp 0.0ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 7.8684807256235825ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 15.736961451247165ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 23.605442176870746ms with RTP over UDP to 127.0.0.1:1337
...
Sent samples at timestamp 19175.487528344514ms with RTP over UDP to 127.0.0.1:1337
Sent samples at timestamp 19183.35600907014ms with RTP over UDP to 127.0.0.1:1337
End... set the marker bit
Sent samples at timestamp 19191.224489795764ms with RTP over UDP to 127.0.0.1:1337
```

### testing packet loss concealment

I my [XDP tool](https://github.com/sevagh/ape) to intercept and randomly drop ~10% of UDP packets, and ran a sender + jitter receiver to test the PLC. The audio sounds choppy, given that waveform correction is not perfect, but plays in its entirety:

```
Yielding audio slice for sequence 2436, timestamp 19167.619047619046ms
Yielding audio slice for sequence 2437, timestamp 19167.619047619046ms
Yielding audio slice for sequence 2438, timestamp 19167.619047619046ms
Yielding audio slice for sequence 2439, timestamp 19191.224489795917ms
audio done, exiting program
Jitter stream stats: "corrected 0 out-of-order packets, concealed 244 lost packets"
```

We can see from the timestamps above that seqs 2438 and 2437 are copies of the original/correct seq 2436.

### testing jitter correction

Similar to the PLC testing, I used my XDP tool ape to scramble 2%\* of incoming UDP packets on the jitters example port, which tests both the jitter correction and PLC (since the last packet might be received out of order):

```
Yielding audio slice for sequence 2439, timestamp 19191.224489795917ms
audio done, exiting program
Jitter stream stats: "corrected 11 out-of-order packets, concealed 61 lost packets"
```

The audio sounds coherent, but again with some glitchiness from the imperfect PLC.

\*: In reality, more than 2%, since there's a feedback scrambling effect and packets can get randomly delayed multiple times
