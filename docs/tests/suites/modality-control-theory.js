// Control Theory — mirrors control_feedback.rs PID logic (pure JS).

export function register(runner) {
    // Stateless PID step: returns { output, new_error, new_integral }
    function pidStep(setpoint, currentValue, prevError, integral, kp, ki, kd, dt) {
        const error = setpoint - currentValue;
        const derivative = (error - prevError) / dt;
        const newIntegral = integral + error * dt;
        const output = kp * error + ki * newIntegral + kd * derivative;
        return { output, new_error: error, new_integral: newIntegral };
    }

    runner.describe('Modality: Control Theory', () => {

        runner.describe('PID step arithmetic', () => {

            runner.it('P-only: output = kp × error', () => {
                const r = pidStep(100, 80, 0, 0, 0.5, 0, 0, 1.0);
                runner.expect(r.output).toBe(10);   // 0.5 × 20
                runner.expect(r.new_error).toBe(20);
            });

            runner.it('zero error yields zero output', () => {
                const r = pidStep(100, 100, 0, 0, 0.5, 0.1, 0.05, 1.0);
                runner.expect(r.output).toBe(0);
            });

            runner.it('integral accumulates across steps', () => {
                const r1 = pidStep(100, 90, 0, 0, 0, 0.1, 0, 1.0);
                const r2 = pidStep(100, 90, r1.new_error, r1.new_integral, 0, 0.1, 0, 1.0);
                // Step 1 integral = 10; step 2 output = 0.1 × 20 = 2 (twice the integral)
                runner.expect(r2.output).toBeGreaterThan(r1.output);
            });

            runner.it('derivative term damps oscillation (output less than P-only when error shrinking)', () => {
                // Error shrinking: prev_error > current error
                const pOnly  = pidStep(100, 90, 0,  0, 0.5, 0, 0,   1.0);
                const withPD = pidStep(100, 90, 15, 0, 0.5, 0, 0.5, 1.0);
                // D term is negative when error falling → damps output
                runner.expect(withPD.output).toBeLessThan(pOnly.output);
            });

            runner.it('converges toward setpoint over 10 closed-loop steps', () => {
                let value = 80, error = 0, integral = 0;
                for (let i = 0; i < 10; i++) {
                    const r = pidStep(100, value, error, integral, 0.5, 0.1, 0.05, 1.0);
                    value += r.output;
                    error    = r.new_error;
                    integral = r.new_integral;
                }
                runner.expect(value).toBeGreaterThan(90);
            });
        });

        runner.describe('PidParameters presets (JS mirrors of Rust constants)', () => {

            // Mirrors PidParameters::conservative_power_system()
            const CONSERVATIVE = { kp: 0.5, ki: 0.1, kd: 0.05 };
            // Mirrors PidParameters::aggressive_response()
            const AGGRESSIVE   = { kp: 2.0, ki: 0.5, kd: 0.2 };

            runner.it('conservative_power_system kp < aggressive kp', () => {
                runner.expect(CONSERVATIVE.kp).toBeLessThan(AGGRESSIVE.kp);
            });

            runner.it('conservative: small step toward setpoint', () => {
                const r = pidStep(100, 80, 0, 0, CONSERVATIVE.kp, CONSERVATIVE.ki, CONSERVATIVE.kd, 1.0);
                runner.expect(r.output).toBeGreaterThan(0);
                runner.expect(r.output).toBeLessThan(20);   // conservative = stays bounded
            });

            runner.it('aggressive: larger initial correction than conservative', () => {
                const c = pidStep(100, 80, 0, 0, CONSERVATIVE.kp, 0, 0, 1.0);
                const a = pidStep(100, 80, 0, 0, AGGRESSIVE.kp,   0, 0, 1.0);
                runner.expect(a.output).toBeGreaterThan(c.output);
            });
        });
    });
}

export default register;
