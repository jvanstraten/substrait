// SPDX-License-Identifier: Apache-2.0

package io.substrait.relation;

import io.substrait.expression.Expression;
import io.substrait.type.Type;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;
import org.immutables.value.Value;

@Value.Immutable
public abstract class Fetch extends SingleInputRel {
  static final org.slf4j.Logger logger =
      org.slf4j.LoggerFactory.getLogger(Fetch.class);

  public abstract long getOffset();
  public abstract OptionalLong getCount();

  @Override
  protected Type.Struct deriveRecordType() {
    return getInput().getRecordType();
  }

  @Override
  public <O, E extends Exception> O accept(RelVisitor<O, E> visitor) throws E {
    return visitor.visit(this);
  }

  public static ImmutableFetch.Builder builder() {
    return ImmutableFetch.builder();
  }
}