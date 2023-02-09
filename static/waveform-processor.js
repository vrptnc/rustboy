class WaveformProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.data = []
    }

    static get parameterDescriptors() {
        return [
            {
                name: "trigger",
                defaultValue: 0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "gain",
                defaultValue: 1,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "frequency",
                defaultValue: 200,
                minValue: 1,
                maxValue: 20000,
                automationRate: "k-rate",
            },
            {
                name: "dutyCycle",
                defaultValue: 0.5,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
        ];
    }

    process(inputs, outputs, parameters) {
        const enabled = parameters.trigger[0] > 0.5
        if (enabled) {
            const samplesPerWavelength = Math.floor(sampleRate / parameters.frequency[0])
            const highSamplesPerWavelength = Math.floor(samplesPerWavelength * parameters.dutyCycle[0])
            const output = outputs[0];
            output.forEach((channel) => {
                for (let i = 0; i < channel.length; i++) {
                    if (this.currentSample > samplesPerWavelength) {
                        this.currentSample = 0
                    }
                    const value = this.currentSample < highSamplesPerWavelength ? -1 : 1;
                    channel[i] = value * parameters.gain[0]
                    this.currentSample++
                }
            });
        }
        return true;
    }
}

registerProcessor("waveform-processor", WaveformProcessor);