const AUDIO_CHANNELS = 2;

class GBAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.head = 0;
    this.samplesSinceLastReport = 0;
    this.underrunsSinceLastReport = 0;
    this.currentLeft = 0.0;
    this.currentRight = 0.0;

    this.port.onmessage = (event) => {
      const data = event.data;
      if (!data) {
        return;
      }

      if (data.type === "samples") {
        this.pushSamples(data.samples);
      } else if (data.type === "reset") {
        this.queue.length = 0;
        this.head = 0;
        this.samplesSinceLastReport = 0;
        this.underrunsSinceLastReport = 0;
      }
    };
  }

  pushSamples(samples) {
    if (!samples || samples.length === 0) {
      return;
    }

    if (samples instanceof Float32Array) {
      this.queue.push(samples);
    } else {
      this.queue.push(new Float32Array(samples));
    }
  }

  popFrame() {
    while (this.queue.length > 0) {
      const front = this.queue[0];
      if ((front.length - this.head) >= AUDIO_CHANNELS) {
        this.currentLeft = front[this.head];
        this.currentRight = front[this.head + 1];
        this.head += AUDIO_CHANNELS;
        return true;
      }
      this.queue.shift();
      this.head = 0;
    }

    return false;
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    if (!output || output.length === 0) {
      return true;
    }

    const leftChannel = output[0];
    const rightChannel = output.length > 1 ? output[1] : null;
    for (let i = 0; i < leftChannel.length; i += 1) {
      if (!this.popFrame()) {
        leftChannel[i] = 0.0;
        if (rightChannel) {
          rightChannel[i] = 0.0;
        }
        this.underrunsSinceLastReport += 1;
      } else {
        leftChannel[i] = this.currentLeft;
        if (rightChannel) {
          rightChannel[i] = this.currentRight;
        }
      }
    }

    for (let channel = 2; channel < output.length; channel += 1) {
      output[channel].set(rightChannel ?? leftChannel);
    }

    this.samplesSinceLastReport += leftChannel.length;
    if (this.samplesSinceLastReport >= 128) {
      this.port.postMessage({
        type: "consumed",
        samples: this.samplesSinceLastReport,
        underruns: this.underrunsSinceLastReport,
      });
      this.samplesSinceLastReport = 0;
      this.underrunsSinceLastReport = 0;
    }

    return true;
  }
}

registerProcessor("gb-audio-processor", GBAudioProcessor);
