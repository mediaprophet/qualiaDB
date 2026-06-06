import { useState, useEffect } from 'react';
import { Users, ShieldCheck, FileSignature, RefreshCw, Key, Share2, Shield, PlusCircle, AlertTriangle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

// Types matching Tauri Backend
interface FrontDoor {
  id: string;
  did_uri: string;
  label: string;
  created_at: string;
}

interface Actor {
  id: string;
  actor_type: string;
  name: string;
  organization: string | null;
  qualifications: string[];
  roles: string[];
  verification_status: string;
  pairwise_did: string;
  root_did_uri: string | null;
}

interface DelegationRule {
  id: string;
  actor_id: string;
  granted_roles: string[];
  legal_basis: string;
  privacy_mode_limit: string;
  allowed_record_types: string[];
  restricted_records: string[];
  is_active: boolean;
}

export default function AgentDirectory() {
  const [activeTab, setActiveTab] = useState<'FRONT_DOOR' | 'ACTORS' | 'DELEGATIONS'>('FRONT_DOOR');
  
  // State
  const [frontDoors, setFrontDoors] = useState<FrontDoor[]>([]);
  const [actors, setActors] = useState<Actor[]>([]);
  const [rules, setRules] = useState<DelegationRule[]>([]);

  // Form State
  const [newDoorLabel, setNewDoorLabel] = useState('');
  
  // Load data
  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const fd = await invoke<FrontDoor[]>('get_front_doors');
      setFrontDoors(fd);
      const ac = await invoke<Actor[]>('get_directory_actors');
      setActors(ac);
      const ru = await invoke<DelegationRule[]>('get_delegation_rules');
      setRules(ru);
    } catch (e) {
      console.error(e);
    }
  };

  const handleGenerateFrontDoor = async () => {
    if (!newDoorLabel) return;
    try {
      await invoke('generate_front_door', { label: newDoorLabel });
      setNewDoorLabel('');
      await loadData();
    } catch (e) {
      console.error(e);
    }
  };

  const handleSyncFoaf = async () => {
    // Mocking the FOAF transformation via Webizen
    const mockActor: Actor = {
      id: `actor-${Date.now()}`,
      actor_type: 'PRACTITIONER',
      name: 'Dr. Alice FOAF-Imported',
      organization: 'General Clinic',
      qualifications: ['M.D.', 'Webizen Verified'],
      roles: ['Primary Care'],
      verification_status: 'VERIFIED',
      pairwise_did: `did:qualia:pairwise:${Date.now().toString(16)}`,
      root_did_uri: 'did:web:generalclinic.org:alice'
    };
    try {
      await invoke('add_directory_actor', { actor: mockActor });
      await loadData();
    } catch (e) {
      console.error(e);
    }
  };

  const handleAddDelegationRule = async (actorId: string) => {
    const newRule: DelegationRule = {
      id: `rule-${Date.now()}`,
      actor_id: actorId,
      granted_roles: ['read_clinical_records'],
      legal_basis: 'Explicit Consent',
      privacy_mode_limit: 'MODE_B_PRIVILEGED',
      allowed_record_types: ['PathologyObservation'],
      restricted_records: ['PsychiatryObservation'],
      is_active: true
    };
    try {
      await invoke('add_delegation_rule', { rule: newRule });
      await loadData();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#0a0a0a] text-white">
      {/* Header */}
      <div className="flex flex-col gap-2 mb-6">
        <h2 className="text-2xl font-bold text-white flex items-center gap-2 border-l-4 border-[#7c3aed] pl-3">
          <Users className="text-[#7c3aed] w-6 h-6" /> Agent Directory & Delegation Manager
        </h2>
        <p className="text-sm text-gray-400 pl-4">
          Manage identity routing, verified Solid FOAF contacts, and granular access delegations bound by Legal Consents and Privacy Modes.
        </p>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-white/10 mb-6">
        <button 
          onClick={() => setActiveTab('FRONT_DOOR')}
          className={`px-4 py-2 text-sm font-bold uppercase tracking-widest transition-all border-b-2 ${activeTab === 'FRONT_DOOR' ? 'border-[#00f0ff] text-[#00f0ff]' : 'border-transparent text-gray-500 hover:text-white'}`}
        >
          <Share2 className="inline w-4 h-4 mr-2" /> Front Doors
        </button>
        <button 
          onClick={() => setActiveTab('ACTORS')}
          className={`px-4 py-2 text-sm font-bold uppercase tracking-widest transition-all border-b-2 ${activeTab === 'ACTORS' ? 'border-[#b026ff] text-[#b026ff]' : 'border-transparent text-gray-500 hover:text-white'}`}
        >
          <Users className="inline w-4 h-4 mr-2" /> Actor Catalog
        </button>
        <button 
          onClick={() => setActiveTab('DELEGATIONS')}
          className={`px-4 py-2 text-sm font-bold uppercase tracking-widest transition-all border-b-2 ${activeTab === 'DELEGATIONS' ? 'border-[#00ff88] text-[#00ff88]' : 'border-transparent text-gray-500 hover:text-white'}`}
        >
          <Shield className="inline w-4 h-4 mr-2" /> Delegation Policies
        </button>
      </div>

      <div className="flex-1 overflow-y-auto pr-2">
        {/* FRONT DOORS TAB */}
        {activeTab === 'FRONT_DOOR' && (
          <div className="flex flex-col gap-6">
            <div className="glass-panel p-5 bg-black/40 border border-[#00f0ff]/20">
              <h3 className="text-lg font-bold text-white mb-2 flex items-center gap-2">
                <PlusCircle className="text-[#00f0ff] w-5 h-5" /> Generate Front Door DID
              </h3>
              <p className="text-xs text-gray-400 mb-4">
                DIDs are contextual identifiers. You can create multiple Front Doors to route different aspects of your life (e.g. Public, Clinical, Dating).
              </p>
              <div className="flex gap-2">
                <input 
                  type="text" 
                  placeholder="Label (e.g. Anonymous Health Forum)" 
                  value={newDoorLabel}
                  onChange={e => setNewDoorLabel(e.target.value)}
                  className="flex-1 bg-black/50 border border-white/10 rounded px-3 py-2 text-sm text-white focus:border-[#00f0ff] outline-none"
                />
                <button 
                  onClick={handleGenerateFrontDoor}
                  className="bg-[#00f0ff]/10 hover:bg-[#00f0ff]/20 text-[#00f0ff] px-4 py-2 rounded text-xs font-bold uppercase tracking-widest border border-[#00f0ff]/30"
                >
                  Generate
                </button>
              </div>
            </div>

            <div className="flex flex-col gap-3">
              {frontDoors.map(fd => (
                <div key={fd.id} className="bg-black/40 border border-white/5 p-4 rounded flex items-center justify-between">
                  <div>
                    <div className="text-sm font-bold text-white mb-1">{fd.label}</div>
                    <div className="text-[10px] text-[#00f0ff] font-mono">{fd.did_uri}</div>
                  </div>
                  <div className="text-xs text-gray-500 font-mono">
                    ID: {fd.id}
                  </div>
                </div>
              ))}
              {frontDoors.length === 0 && (
                <div className="text-center text-gray-500 text-xs py-8">No Front Doors generated.</div>
              )}
            </div>
          </div>
        )}

        {/* ACTORS TAB */}
        {activeTab === 'ACTORS' && (
          <div className="flex flex-col gap-6">
            <div className="flex justify-between items-center bg-black/40 border border-white/10 p-4 rounded">
              <div className="text-sm text-gray-300">
                Transform standard Solid FOAF graphs into Cryptographic Webizen Actors.
              </div>
              <button 
                onClick={handleSyncFoaf}
                className="bg-[#b026ff]/10 hover:bg-[#b026ff]/20 text-[#b026ff] px-4 py-2 rounded text-xs font-bold uppercase tracking-widest flex items-center gap-2 border border-[#b026ff]/30"
              >
                <RefreshCw className="w-4 h-4" /> Sync Solid FOAF
              </button>
            </div>

            <div className="flex flex-col gap-4">
              {actors.map(actor => (
                <div key={actor.id} className="bg-[#111] border-l-4 border-[#b026ff] p-5 rounded flex flex-col gap-3 shadow-lg">
                  <div className="flex justify-between items-start">
                    <div>
                      <h4 className="text-lg font-bold text-white flex items-center gap-2">
                        {actor.name}
                        <span className="text-[9px] bg-[#b026ff]/20 text-[#b026ff] px-2 py-0.5 rounded uppercase tracking-widest">{actor.actor_type}</span>
                      </h4>
                      {actor.organization && <div className="text-xs text-gray-400 mt-1">{actor.organization}</div>}
                    </div>
                    {actor.verification_status === 'VERIFIED' ? (
                      <span className="text-[10px] text-[#10b981] font-bold uppercase tracking-widest flex items-center gap-1">
                        <ShieldCheck className="w-3 h-3" /> VERIFIED
                      </span>
                    ) : (
                      <span className="text-[10px] text-yellow-500 font-bold uppercase tracking-widest flex items-center gap-1">
                        <AlertTriangle className="w-3 h-3" /> SELF CLAIMED
                      </span>
                    )}
                  </div>
                  
                  <div className="bg-black/40 rounded p-3 text-xs font-mono border border-white/5 flex flex-col gap-1">
                    <div className="flex text-gray-400"><span className="w-24 text-gray-500">Pairwise DID:</span> <span className="text-[#00f0ff]">{actor.pairwise_did}</span></div>
                    {actor.root_did_uri && <div className="flex text-gray-400"><span className="w-24 text-gray-500">Root DID:</span> <span>{actor.root_did_uri}</span></div>}
                  </div>

                  <div className="flex gap-6 mt-2">
                    <div>
                      <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Roles</div>
                      <div className="flex gap-2">
                        {actor.roles.map(r => <span key={r} className="text-xs bg-white/5 px-2 py-1 rounded text-gray-300">{r}</span>)}
                      </div>
                    </div>
                    <div>
                      <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Qualifications</div>
                      <div className="flex gap-2">
                        {actor.qualifications.map(q => <span key={q} className="text-xs italic text-gray-400">{q}</span>)}
                      </div>
                    </div>
                  </div>

                  <div className="flex gap-3 mt-3 border-t border-white/5 pt-4">
                    <button className="flex-1 py-2 text-xs font-bold uppercase tracking-widest bg-white/5 hover:bg-white/10 rounded border border-white/10 flex justify-center items-center gap-2">
                      <Key className="w-4 h-4" /> Sign Shared Values Agreement
                    </button>
                    <button 
                      onClick={() => handleAddDelegationRule(actor.id)}
                      className="flex-1 py-2 text-xs font-bold uppercase tracking-widest bg-[#00ff88]/10 text-[#00ff88] hover:bg-[#00ff88]/20 rounded border border-[#00ff88]/30 flex justify-center items-center gap-2"
                    >
                      <Shield className="w-4 h-4" /> Authorize Delegation
                    </button>
                  </div>
                </div>
              ))}
              {actors.length === 0 && (
                <div className="text-center text-gray-500 text-xs py-8">No actors in directory. Sync FOAF to populate.</div>
              )}
            </div>
          </div>
        )}

        {/* DELEGATIONS TAB */}
        {activeTab === 'DELEGATIONS' && (
          <div className="flex flex-col gap-4">
            {rules.map(rule => {
              const actor = actors.find(a => a.id === rule.actor_id);
              return (
                <div key={rule.id} className="bg-[#111] border-l-4 border-[#00ff88] p-5 rounded shadow-lg flex flex-col gap-3">
                  <div className="flex justify-between items-center border-b border-white/10 pb-3">
                    <h4 className="text-sm font-bold text-white">Rule {rule.id}</h4>
                    <span className="text-[10px] bg-[#10b981]/20 text-[#10b981] px-2 py-1 rounded uppercase tracking-widest font-bold">
                      {rule.is_active ? 'ACTIVE' : 'SUSPENDED'}
                    </span>
                  </div>
                  
                  <div className="text-sm text-gray-300">
                    Delegated to: <span className="font-bold text-white">{actor?.name || rule.actor_id}</span>
                  </div>

                  <div className="grid grid-cols-2 gap-4 mt-2">
                    <div className="bg-black/40 p-3 rounded border border-white/5">
                      <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Legal Basis</div>
                      <div className="text-xs font-mono text-white">{rule.legal_basis}</div>
                    </div>
                    <div className="bg-black/40 p-3 rounded border border-white/5">
                      <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Max Privacy Mode</div>
                      <div className="text-xs font-mono text-[#00ff88]">{rule.privacy_mode_limit}</div>
                    </div>
                  </div>

                  <div className="mt-2 text-xs text-gray-400">
                    <span className="font-bold text-gray-500">Whitelisted Types:</span> {rule.allowed_record_types.join(', ')}<br/>
                    <span className="font-bold text-gray-500">Blacklisted Types:</span> {rule.restricted_records.join(', ')}
                  </div>
                  
                  <div className="flex justify-end mt-2">
                    <button className="text-[10px] text-red-400 hover:text-red-300 uppercase tracking-widest font-bold px-3 py-1 bg-red-400/10 rounded">
                      Revoke Delegation
                    </button>
                  </div>
                </div>
              );
            })}
            {rules.length === 0 && (
              <div className="text-center text-gray-500 text-xs py-8">No active delegation policies.</div>
            )}
          </div>
        )}

      </div>
    </div>
  );
}
