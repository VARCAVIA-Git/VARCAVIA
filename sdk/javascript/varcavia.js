/**
 * VARCAVIA JavaScript SDK — Minimal client for the VERIT Protocol.
 *
 * Works in Node.js (18+) and modern browsers.
 *
 * Usage:
 *   import { Varcavia } from './varcavia.js';
 *   const v = new Varcavia('http://localhost:8080');
 *   const result = await v.verify('Earth diameter is 12742 km');
 *   console.log(result.score);
 */

export class Varcavia {
  /**
   * @param {string} baseUrl - API base URL (default: http://localhost:8080)
   */
  constructor(baseUrl = 'http://localhost:8080') {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  /**
   * Verify a fact and get its Data DNA + score.
   * @param {string} fact - The factual claim to verify.
   * @returns {Promise<Object>} { fact, status, data_dna, score, verification_count, duplicate }
   */
  async verify(fact) {
    const res = await fetch(`${this.baseUrl}/api/v1/verify?fact=${encodeURIComponent(fact)}`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }

  /**
   * Submit data to the network.
   * @param {string} content - The data content.
   * @param {string} domain - Data domain (default: 'general').
   * @param {string} source - Source identifier (default: 'js-sdk').
   * @returns {Promise<string>} The data ID (blake3 hex).
   */
  async submit(content, domain = 'general', source = 'js-sdk') {
    const res = await fetch(`${this.baseUrl}/api/v1/data`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content, domain, source }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    const data = await res.json();
    return data.id;
  }

  /**
   * Get data by ID.
   * @param {string} dataId - The blake3 hex ID.
   * @returns {Promise<Object>} { id, content, domain, score }
   */
  async get(dataId) {
    const res = await fetch(`${this.baseUrl}/api/v1/data/${dataId}`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }

  /**
   * Get the full Data DNA for a datum.
   * @param {string} dataId
   * @returns {Promise<Object>}
   */
  async getDna(dataId) {
    const res = await fetch(`${this.baseUrl}/api/v1/data/${dataId}/dna`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }

  /**
   * Query data by domain.
   * @param {Object} opts - { domain, limit }
   * @returns {Promise<Array>}
   */
  async query({ domain, limit = 20 } = {}) {
    const body = { query: '', limit };
    if (domain) body.domain = domain;
    const res = await fetch(`${this.baseUrl}/api/v1/data/query`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }

  /**
   * Get node statistics.
   * @returns {Promise<Object>}
   */
  async stats() {
    const res = await fetch(`${this.baseUrl}/api/v1/stats`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }

  /**
   * Health check.
   * @returns {Promise<Object>}
   */
  async health() {
    const res = await fetch(`${this.baseUrl}/health`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    return res.json();
  }
}
