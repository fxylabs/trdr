import { Placeholder } from "./Placeholder";

/**
 * What happened since the user last looked.
 *
 * A placeholder. The model behind it is `today.get` in section 9.1, and the
 * track that implements that command is the one that fills this in.
 */
export function Today()
{
    return <Placeholder title="Today" command="today.get" />;
}
