import { Action, ActionType } from '../../actions';
import { finish, start } from './sharedStateManagement';

const DEFAULT: State = {
  requestsInProgress: 0,
};

interface State {
  requestsInProgress: number;
  stdout?: string;
  stderr?: string;
}

export default function russol(state = DEFAULT, action: Action): State {
  switch (action.type) {
    case ActionType.RequestRussol:
      return start(DEFAULT, state);
    case ActionType.RussolSucceeded:
      return finish(state);
    case ActionType.RussolFailed: {
      const { stdout = '', stderr = '' } = action;
      return finish(state, { stdout, stderr });
    }
    default:
      return state;
  }
}
